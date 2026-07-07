use crate::codex::metadata::SessionMetadata;
use crate::codex::paths::{normalize_windows_extended_path, visible_path_string};
use crate::codex::{CommandError, Result, ThreadLifecycle};
use rusqlite::{params, Connection, OptionalExtension};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateEntry {
    pub db_path: String,
    pub exists: bool,
    pub provider: Option<String>,
    pub archived: Option<i32>,
    pub title: Option<String>,
    pub rollout_path: Option<String>,
    pub cwd: Option<String>,
}

pub fn state_dbs(codex_home: &Path) -> Vec<PathBuf> {
    [
        codex_home.join("state_5.sqlite"),
        codex_home.join("sqlite/state_5.sqlite"),
    ]
    .into_iter()
    .filter(|path| path.exists())
    .collect()
}

pub fn state_entry(db: &Path, thread_id: &str) -> StateEntry {
    let db_path = db.display().to_string();
    let Ok(connection) =
        Connection::open_with_flags(db, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
    else {
        return StateEntry {
            db_path,
            exists: false,
            provider: None,
            archived: None,
            title: None,
            rollout_path: None,
            cwd: None,
        };
    };
    let row = connection
        .query_row(
            "select model_provider, archived, title, rollout_path, cwd from threads where id=?1",
            [thread_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i32>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                ))
            },
        )
        .optional();
    match row {
        Ok(Some((provider, archived, title, rollout_path, cwd))) => StateEntry {
            db_path,
            exists: true,
            provider: Some(provider),
            archived: Some(archived),
            title: (!title.trim().is_empty()).then_some(title),
            rollout_path: (!rollout_path.trim().is_empty()).then_some(rollout_path),
            cwd: (!cwd.trim().is_empty()).then_some(cwd),
        },
        _ => StateEntry {
            db_path,
            exists: false,
            provider: None,
            archived: None,
            title: None,
            rollout_path: None,
            cwd: None,
        },
    }
}

pub fn upsert_state_entries(
    codex_home: &Path,
    items: &[(SessionMetadata, ThreadLifecycle)],
    provider: &str,
) -> Result<()> {
    upsert_state_entries_with_provider(codex_home, items, Some(provider))
}

pub fn upsert_state_entries_from_metadata(
    codex_home: &Path,
    items: &[(SessionMetadata, ThreadLifecycle)],
) -> Result<()> {
    upsert_state_entries_with_provider(codex_home, items, None)
}

fn upsert_state_entries_with_provider(
    codex_home: &Path,
    items: &[(SessionMetadata, ThreadLifecycle)],
    provider_override: Option<&str>,
) -> Result<()> {
    for db in state_dbs(codex_home) {
        let connection = Connection::open(&db)
            .map_err(|error| CommandError::new("sqlite_open_failed", error.to_string()))?;
        connection
            .busy_timeout(std::time::Duration::from_millis(5_000))
            .map_err(|error| CommandError::new("sqlite_busy_timeout_failed", error.to_string()))?;
        for (metadata, lifecycle) in items {
            let archived = archived_value(lifecycle);
            let rollout_path = visible_path_string(&metadata.path);
            let cwd = normalize_windows_extended_path(&metadata.cwd);
            let provider = provider_override
                .or(metadata.provider.as_deref())
                .filter(|value| !value.trim().is_empty())
                .unwrap_or("unknown");
            let exists: i64 = connection
                .query_row(
                    "select count(*) from threads where id=?1",
                    [&metadata.thread_id],
                    |row| row.get(0),
                )
                .map_err(|error| CommandError::new("sqlite_query_failed", error.to_string()))?;
            if exists > 0 {
                connection
                    .execute(
                        "
                        update threads
                        set model_provider=?1, archived=?2, archived_at=CASE WHEN ?2 = 0 THEN NULL ELSE archived_at END, title=?3,
                            first_user_message=?4, preview=?5, updated_at=?6, updated_at_ms=?7,
                            rollout_path=?8, cwd=?9
                        where id=?10
                        ",
                        params![
                            provider,
                            archived,
                            &metadata.title,
                            &metadata.first_user_message,
                            &metadata.preview,
                            metadata.updated_at_ms / 1000,
                            metadata.updated_at_ms,
                            &rollout_path,
                            &cwd,
                            &metadata.thread_id,
                        ],
                    )
                    .map_err(|error| {
                        CommandError::new("sqlite_update_failed", error.to_string())
                    })?;
            } else {
                connection
                    .execute(
                        "
                        insert into threads (
                            id, rollout_path, created_at, updated_at, source, model_provider, cwd,
                            title, sandbox_policy, approval_mode, tokens_used, has_user_event,
                            archived, archived_at, git_sha, git_branch, git_origin_url, cli_version,
                            first_user_message, agent_nickname, agent_role, memory_mode, model,
                            reasoning_effort, agent_path, created_at_ms, updated_at_ms,
                            thread_source, preview
                        ) values (
                            ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 0, 0, ?11, NULL, NULL, NULL,
                            NULL, ?12, ?13, NULL, NULL, 'enabled', NULL, NULL, NULL, ?14, ?15,
                            ?16, ?17
                        )
                        ",
                        params![
                            &metadata.thread_id,
                            &rollout_path,
                            metadata.created_at_ms / 1000,
                            metadata.updated_at_ms / 1000,
                            &metadata.source,
                            provider,
                            &cwd,
                            &metadata.title,
                            "{\"type\":\"disabled\"}",
                            "never",
                            archived,
                            &metadata.cli_version,
                            &metadata.first_user_message,
                            metadata.created_at_ms,
                            metadata.updated_at_ms,
                            metadata.thread_source.as_deref(),
                            &metadata.preview,
                        ],
                    )
                    .map_err(|error| {
                        CommandError::new("sqlite_insert_failed", error.to_string())
                    })?;
            }
        }
    }
    Ok(())
}

pub fn delete_state_entries(codex_home: &Path, thread_ids: &[String]) -> Result<()> {
    for db in state_dbs(codex_home) {
        let connection = Connection::open(&db)
            .map_err(|error| CommandError::new("sqlite_open_failed", error.to_string()))?;
        connection
            .busy_timeout(std::time::Duration::from_millis(5_000))
            .map_err(|error| CommandError::new("sqlite_busy_timeout_failed", error.to_string()))?;
        for thread_id in thread_ids {
            connection
                .execute("delete from threads where id=?1", [thread_id])
                .map_err(|error| CommandError::new("sqlite_delete_failed", error.to_string()))?;
        }
    }
    Ok(())
}

pub fn update_archive_state(
    codex_home: &Path,
    updates: &[(String, PathBuf)],
    lifecycle: &ThreadLifecycle,
) -> Result<()> {
    let archived = archived_value(lifecycle);
    let archived_at = match lifecycle {
        ThreadLifecycle::Active => None,
        ThreadLifecycle::Archived => Some(chrono::Utc::now().timestamp()),
    };
    for db in state_dbs(codex_home) {
        let connection = Connection::open(&db)
            .map_err(|error| CommandError::new("sqlite_open_failed", error.to_string()))?;
        connection
            .busy_timeout(std::time::Duration::from_millis(5_000))
            .map_err(|error| CommandError::new("sqlite_busy_timeout_failed", error.to_string()))?;
        for (thread_id, rollout_path) in updates {
            connection
                .execute(
                    "
                    update threads
                    set archived=?1, archived_at=?2, rollout_path=?3
                    where id=?4
                    ",
                    params![
                        archived,
                        archived_at,
                        visible_path_string(rollout_path),
                        thread_id
                    ],
                )
                .map_err(|error| CommandError::new("sqlite_update_failed", error.to_string()))?;
        }
    }
    Ok(())
}

fn archived_value(lifecycle: &ThreadLifecycle) -> i32 {
    match lifecycle {
        ThreadLifecycle::Active => 0,
        ThreadLifecycle::Archived => 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codex::test_support::{init_state_db, insert_state_row_with_title};
    use rusqlite::Connection;

    fn metadata_with_extended_paths(thread_id: &str) -> SessionMetadata {
        SessionMetadata {
            thread_id: thread_id.to_string(),
            provider: Some("funai".to_string()),
            created_at_ms: 1_783_265_504_000,
            updated_at_ms: 1_783_275_940_813,
            cwd: r"\\?\D:\dev\AI\AIPro\fun-claw".to_string(),
            source: "vscode".to_string(),
            cli_version: "0.140.0".to_string(),
            thread_source: Some("user".to_string()),
            title: "slack1".to_string(),
            first_user_message: "first message".to_string(),
            preview: "first message".to_string(),
            path: PathBuf::from(r"\\?\C:\Users\jianrui\.codex\sessions\2026\07\05\rollout-a.jsonl"),
        }
    }

    #[test]
    fn upsert_state_entries_normalizes_extended_windows_paths_when_inserting() {
        let temp = tempfile::tempdir().unwrap();
        let codex = temp.path().join(".codex");
        let state_db = codex.join("state_5.sqlite");
        let thread_id = "019f32e8-178a-7b01-9a43-61e5a75d73ae";
        init_state_db(&state_db);

        upsert_state_entries_from_metadata(
            &codex,
            &[(
                metadata_with_extended_paths(thread_id),
                ThreadLifecycle::Active,
            )],
        )
        .unwrap();

        let connection = Connection::open(state_db).unwrap();
        let row: (String, String) = connection
            .query_row(
                "select rollout_path, cwd from threads where id=?1",
                [thread_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(
            row,
            (
                r"C:\Users\jianrui\.codex\sessions\2026\07\05\rollout-a.jsonl".to_string(),
                r"D:\dev\AI\AIPro\fun-claw".to_string(),
            )
        );
    }

    #[test]
    fn upsert_state_entries_corrects_existing_rollout_path_and_cwd() {
        let temp = tempfile::tempdir().unwrap();
        let codex = temp.path().join(".codex");
        let state_db = codex.join("state_5.sqlite");
        let thread_id = "019f32e8-178a-7b01-9a43-61e5a75d73ae";
        init_state_db(&state_db);
        insert_state_row_with_title(&state_db, thread_id, "funai", 0, "old title");
        let connection = Connection::open(&state_db).unwrap();
        connection
            .execute(
                "update threads set rollout_path=?1, cwd=?2 where id=?3",
                (
                    r"\\?\C:\Users\jianrui\.codex\sessions\bad.jsonl",
                    r"\\?\D:\dev\AI\AIPro\bad",
                    thread_id,
                ),
            )
            .unwrap();

        upsert_state_entries_from_metadata(
            &codex,
            &[(
                metadata_with_extended_paths(thread_id),
                ThreadLifecycle::Active,
            )],
        )
        .unwrap();

        let row: (String, String, String) = connection
            .query_row(
                "select rollout_path, cwd, title from threads where id=?1",
                [thread_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        assert_eq!(
            row,
            (
                r"C:\Users\jianrui\.codex\sessions\2026\07\05\rollout-a.jsonl".to_string(),
                r"D:\dev\AI\AIPro\fun-claw".to_string(),
                "slack1".to_string(),
            )
        );
    }
}

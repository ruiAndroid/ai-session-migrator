use crate::codex::metadata::SessionMetadata;
use crate::codex::{CommandError, Result, ThreadLifecycle};
use rusqlite::{params, Connection, OptionalExtension};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateEntry {
    pub db_path: String,
    pub exists: bool,
    pub provider: Option<String>,
    pub archived: Option<i32>,
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
        };
    };
    let row = connection
        .query_row(
            "select model_provider, archived from threads where id=?1",
            [thread_id],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, i32>(1)?)),
        )
        .optional();
    match row {
        Ok(Some((provider, archived))) => StateEntry {
            db_path,
            exists: true,
            provider: Some(provider),
            archived: Some(archived),
        },
        _ => StateEntry {
            db_path,
            exists: false,
            provider: None,
            archived: None,
        },
    }
}

pub fn upsert_state_entries(
    codex_home: &Path,
    items: &[(SessionMetadata, ThreadLifecycle)],
    provider: &str,
) -> Result<()> {
    for db in state_dbs(codex_home) {
        let connection = Connection::open(&db)
            .map_err(|error| CommandError::new("sqlite_open_failed", error.to_string()))?;
        connection
            .busy_timeout(std::time::Duration::from_millis(5_000))
            .map_err(|error| CommandError::new("sqlite_busy_timeout_failed", error.to_string()))?;
        for (metadata, lifecycle) in items {
            let archived = archived_value(lifecycle);
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
                            first_user_message=?4, preview=?5, updated_at=?6, updated_at_ms=?7
                        where id=?8
                        ",
                        params![
                            provider,
                            archived,
                            &metadata.title,
                            &metadata.first_user_message,
                            &metadata.preview,
                            metadata.updated_at_ms / 1000,
                            metadata.updated_at_ms,
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
                            metadata.path.display().to_string(),
                            metadata.created_at_ms / 1000,
                            metadata.updated_at_ms / 1000,
                            &metadata.source,
                            provider,
                            &metadata.cwd,
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

fn archived_value(lifecycle: &ThreadLifecycle) -> i32 {
    match lifecycle {
        ThreadLifecycle::Active => 0,
        ThreadLifecycle::Archived => 1,
    }
}

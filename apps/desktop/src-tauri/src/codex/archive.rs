use crate::codex::backup::create_backup_dir;
use crate::codex::metadata::{metadata_from_bytes, SessionMetadata};
use crate::codex::scan::session_files;
use crate::codex::sqlite::{state_dbs, update_archive_state};
use crate::codex::{ArchiveRequest, ArchiveResult, CommandError, Result, ThreadLifecycle};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

pub fn apply_archive_sessions(request: ArchiveRequest) -> Result<ArchiveResult> {
    change_archive_state(request, ThreadLifecycle::Archived)
}

pub fn apply_activate_sessions(request: ArchiveRequest) -> Result<ArchiveResult> {
    change_archive_state(request, ThreadLifecycle::Active)
}

fn change_archive_state(
    request: ArchiveRequest,
    target_lifecycle: ThreadLifecycle,
) -> Result<ArchiveResult> {
    if request.thread_ids.is_empty() {
        return Err(CommandError::new(
            "no_session_selected",
            "Select at least one session to update.",
        ));
    }
    let codex_home = PathBuf::from(&request.codex_home);
    if !codex_home.exists() {
        return Err(CommandError::new(
            "codex_home_missing",
            format!("Codex home does not exist: {}", codex_home.display()),
        ));
    }

    let selected: BTreeSet<String> = request.thread_ids.into_iter().collect();
    let mut found_selected = BTreeSet::new();
    let mut items = Vec::new();
    for file in session_files(&codex_home)? {
        let raw = fs::read(&file.path)
            .map_err(|error| CommandError::io("read session", file.path.display(), error))?;
        let metadata = metadata_from_bytes(&raw, &file.path)?;
        if !selected.contains(&metadata.thread_id) {
            continue;
        }
        found_selected.insert(metadata.thread_id.clone());
        if file.lifecycle == target_lifecycle {
            return Err(CommandError::new(
                archive_state_error_code(&target_lifecycle),
                format!(
                    "Session {} is already {}.",
                    metadata.thread_id,
                    lifecycle_name(&target_lifecycle)
                ),
            ));
        }
        items.push((metadata, file.lifecycle, file.path));
    }

    if let Some(missing) = selected.difference(&found_selected).next() {
        return Err(CommandError::new(
            "selected_thread_missing",
            format!("Selected thread no longer exists: {missing}"),
        ));
    }

    let moved_threads = items
        .iter()
        .map(|(metadata, _, _)| metadata.thread_id.clone())
        .collect::<Vec<_>>();
    let backup_inputs = backup_inputs(&codex_home, &items);
    let backup_dir = create_backup_dir(&codex_home, &backup_inputs)?;
    let mut updates = Vec::new();
    for (metadata, _, source_path) in &items {
        let target_path = target_session_path(&codex_home, metadata, &target_lifecycle);
        if let Some(parent) = target_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| CommandError::io("create session parent", parent.display(), error))
                .map_err(|error| {
                    CommandError::post_backup(backup_dir.display(), "create session parent", error)
                })?;
        }
        if target_path.exists() {
            return Err(CommandError::post_backup(
                backup_dir.display(),
                "move session",
                CommandError::new(
                    "target_session_exists",
                    format!("Target session already exists: {}", target_path.display()),
                ),
            ));
        }
        fs::rename(source_path, &target_path)
            .map_err(|error| CommandError::io("move session", source_path.display(), error))
            .map_err(|error| {
                CommandError::post_backup(backup_dir.display(), "move session", error)
            })?;
        updates.push((metadata.thread_id.clone(), target_path));
    }
    update_archive_state(&codex_home, &updates, &target_lifecycle).map_err(|error| {
        CommandError::post_backup(backup_dir.display(), "update sqlite archive state", error)
    })?;

    Ok(ArchiveResult {
        changed_threads: moved_threads,
        backup_dir: Some(backup_dir.display().to_string()),
    })
}

fn archive_state_error_code(target_lifecycle: &ThreadLifecycle) -> &'static str {
    match target_lifecycle {
        ThreadLifecycle::Active => "activate_requires_archived_sessions",
        ThreadLifecycle::Archived => "archive_requires_active_sessions",
    }
}

fn lifecycle_name(lifecycle: &ThreadLifecycle) -> &'static str {
    match lifecycle {
        ThreadLifecycle::Active => "active",
        ThreadLifecycle::Archived => "archived",
    }
}

fn backup_inputs(
    codex_home: &Path,
    items: &[(SessionMetadata, ThreadLifecycle, PathBuf)],
) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = items.iter().map(|(_, _, path)| path.clone()).collect();
    files.extend(state_dbs(codex_home));
    files
}

fn target_session_path(
    codex_home: &Path,
    metadata: &SessionMetadata,
    target_lifecycle: &ThreadLifecycle,
) -> PathBuf {
    let file_name = metadata
        .path
        .file_name()
        .map(|name| name.to_owned())
        .unwrap_or_else(|| format!("rollout-{}.jsonl", metadata.thread_id).into());
    match target_lifecycle {
        ThreadLifecycle::Archived => codex_home.join("archived_sessions").join(file_name),
        ThreadLifecycle::Active => {
            let timestamp = chrono::DateTime::from_timestamp_millis(metadata.created_at_ms)
                .unwrap_or_else(chrono::Utc::now);
            codex_home
                .join("sessions")
                .join(timestamp.format("%Y").to_string())
                .join(timestamp.format("%m").to_string())
                .join(timestamp.format("%d").to_string())
                .join(file_name)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codex::test_support::{init_state_db, insert_state_row, write_jsonl};
    use rusqlite::Connection;

    #[test]
    fn archive_moves_active_session_to_archived_sessions_and_updates_state() {
        let temp = tempfile::tempdir().unwrap();
        let codex = temp.path().join(".codex");
        let thread_id = "019eca3b-941d-7340-9b14-328c635a6523";
        let active_path = codex
            .join("sessions")
            .join("2026")
            .join("06")
            .join("15")
            .join("rollout-a-019eca3b-941d-7340-9b14-328c635a6523.jsonl");
        let archived_path = codex
            .join("archived_sessions")
            .join("rollout-a-019eca3b-941d-7340-9b14-328c635a6523.jsonl");
        write_jsonl(&active_path, thread_id, "funai", false, "active session");
        let db = codex.join("state_5.sqlite");
        init_state_db(&db);
        insert_state_row(&db, thread_id, "funai", 0);

        let result = apply_archive_sessions(ArchiveRequest {
            codex_home: codex.display().to_string(),
            thread_ids: vec![thread_id.to_string()],
        })
        .unwrap();

        assert_eq!(result.changed_threads, vec![thread_id.to_string()]);
        assert!(!active_path.exists());
        assert!(archived_path.exists());
        assert!(Path::new(&result.backup_dir.unwrap())
            .join("sessions/2026/06/15/rollout-a-019eca3b-941d-7340-9b14-328c635a6523.jsonl")
            .exists());
        let connection = Connection::open(&db).unwrap();
        let (archived, archived_at, rollout_path): (i32, Option<i64>, String) = connection
            .query_row(
                "select archived, archived_at, rollout_path from threads where id=?1",
                [thread_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        assert_eq!(archived, 1);
        assert!(archived_at.is_some());
        assert_eq!(rollout_path, archived_path.display().to_string());
    }

    #[test]
    fn activate_moves_archived_session_to_dated_sessions_dir_and_updates_state() {
        let temp = tempfile::tempdir().unwrap();
        let codex = temp.path().join(".codex");
        let thread_id = "019eca3b-941d-7340-9b14-328c635a6523";
        let archived_path = codex
            .join("archived_sessions")
            .join("rollout-a-019eca3b-941d-7340-9b14-328c635a6523.jsonl");
        let active_path = codex
            .join("sessions")
            .join("2026")
            .join("06")
            .join("15")
            .join("rollout-a-019eca3b-941d-7340-9b14-328c635a6523.jsonl");
        write_jsonl(
            &archived_path,
            thread_id,
            "funai",
            false,
            "archived session",
        );
        let db = codex.join("state_5.sqlite");
        init_state_db(&db);
        insert_state_row(&db, thread_id, "funai", 1);

        let result = apply_activate_sessions(ArchiveRequest {
            codex_home: codex.display().to_string(),
            thread_ids: vec![thread_id.to_string()],
        })
        .unwrap();

        assert_eq!(result.changed_threads, vec![thread_id.to_string()]);
        assert!(!archived_path.exists());
        assert!(active_path.exists());
        let connection = Connection::open(&db).unwrap();
        let (archived, archived_at, rollout_path): (i32, Option<i64>, String) = connection
            .query_row(
                "select archived, archived_at, rollout_path from threads where id=?1",
                [thread_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        assert_eq!(archived, 0);
        assert!(archived_at.is_none());
        assert_eq!(rollout_path, active_path.display().to_string());
    }
}

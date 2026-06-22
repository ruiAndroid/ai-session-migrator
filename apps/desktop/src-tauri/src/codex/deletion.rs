use crate::codex::backup::create_backup_dir;
use crate::codex::metadata::{metadata_from_bytes, SessionMetadata};
use crate::codex::scan::session_files;
use crate::codex::sqlite::{delete_state_entries, state_dbs};
use crate::codex::{
    CommandError, DeleteArchivedRequest, DeleteArchivedResult, Result, ThreadLifecycle,
};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

pub fn preview_delete_archived_sessions(
    request: DeleteArchivedRequest,
) -> Result<DeleteArchivedResult> {
    delete_archived_sessions(request, false)
}

pub fn apply_delete_archived_sessions(
    request: DeleteArchivedRequest,
) -> Result<DeleteArchivedResult> {
    delete_archived_sessions(request, true)
}

fn delete_archived_sessions(
    request: DeleteArchivedRequest,
    apply: bool,
) -> Result<DeleteArchivedResult> {
    if request.thread_ids.is_empty() {
        return Err(CommandError::new(
            "no_session_selected",
            "Select at least one archived session to delete.",
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
    let mut archived_items = Vec::new();

    for file in session_files(&codex_home)? {
        let raw = fs::read(&file.path)
            .map_err(|error| CommandError::io("read session", file.path.display(), error))?;
        let metadata = metadata_from_bytes(&raw, &file.path)?;
        if !selected.contains(&metadata.thread_id) {
            continue;
        }
        found_selected.insert(metadata.thread_id.clone());
        if file.lifecycle != ThreadLifecycle::Archived {
            return Err(CommandError::new(
                "delete_requires_archived_sessions",
                format!("Refusing to delete active session: {}", metadata.thread_id),
            ));
        }
        archived_items.push((metadata, file.path));
    }

    if let Some(missing) = selected.difference(&found_selected).next() {
        return Err(CommandError::new(
            "selected_thread_missing",
            format!("Selected thread no longer exists: {missing}"),
        ));
    }

    let deleted_threads = archived_items
        .iter()
        .map(|(metadata, _)| metadata.thread_id.clone())
        .collect::<Vec<_>>();
    if !apply {
        return Ok(DeleteArchivedResult {
            deleted_threads,
            backup_dir: None,
            dry_run: true,
        });
    }

    let backup_inputs = backup_inputs(&codex_home, &archived_items);
    let backup_dir = create_backup_dir(&codex_home, &backup_inputs)?;
    for (_, path) in &archived_items {
        fs::remove_file(path)
            .map_err(|error| CommandError::io("delete session", path.display(), error))
            .map_err(|error| {
                CommandError::post_backup(backup_dir.display(), "delete session", error)
            })?;
    }
    delete_state_entries(&codex_home, &deleted_threads).map_err(|error| {
        CommandError::post_backup(
            backup_dir.display(),
            "delete sqlite archived session metadata",
            error,
        )
    })?;

    Ok(DeleteArchivedResult {
        deleted_threads,
        backup_dir: Some(backup_dir.display().to_string()),
        dry_run: false,
    })
}

fn backup_inputs(codex_home: &Path, items: &[(SessionMetadata, PathBuf)]) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = items.iter().map(|(_, path)| path.clone()).collect();
    files.extend(state_dbs(codex_home));
    files
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codex::test_support::{init_state_db, insert_state_row, write_jsonl};
    use rusqlite::Connection;
    use std::fs;
    use std::path::Path;

    #[test]
    fn apply_deletes_only_archived_session_files_after_backup() {
        let temp = tempfile::tempdir().unwrap();
        let codex = temp.path().join(".codex");
        fs::create_dir_all(codex.join("sessions")).unwrap();
        let archived_id = "019eca3b-941d-7340-9b14-328c635a6523";
        let archived_path =
            codex.join("archived_sessions/rollout-a-019eca3b-941d-7340-9b14-328c635a6523.jsonl");
        write_jsonl(
            &archived_path,
            archived_id,
            "funai",
            false,
            "archived session to delete",
        );
        let db = codex.join("state_5.sqlite");
        init_state_db(&db);
        insert_state_row(&db, archived_id, "funai", 1);

        let result = apply_delete_archived_sessions(DeleteArchivedRequest {
            codex_home: codex.display().to_string(),
            thread_ids: vec![archived_id.to_string()],
        })
        .unwrap();

        assert_eq!(result.deleted_threads, vec![archived_id.to_string()]);
        assert!(!result.dry_run);
        assert!(!archived_path.exists());
        let backup_dir = result.backup_dir.unwrap();
        assert!(Path::new(&backup_dir)
            .join("archived_sessions/rollout-a-019eca3b-941d-7340-9b14-328c635a6523.jsonl")
            .exists());
        let connection = Connection::open(&db).unwrap();
        let count: i64 = connection
            .query_row(
                "select count(*) from threads where id=?1",
                [archived_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn apply_refuses_to_delete_active_sessions() {
        let temp = tempfile::tempdir().unwrap();
        let codex = temp.path().join(".codex");
        let active_id = "019ec94d-720d-7a12-a379-28c8042bc6b4";
        let active_path =
            codex.join("sessions/2026/06/15/rollout-a-019ec94d-720d-7a12-a379-28c8042bc6b4.jsonl");
        write_jsonl(&active_path, active_id, "funai", false, "active session");

        let error = apply_delete_archived_sessions(DeleteArchivedRequest {
            codex_home: codex.display().to_string(),
            thread_ids: vec![active_id.to_string()],
        })
        .unwrap_err();

        assert_eq!(error.code, "delete_requires_archived_sessions");
        assert!(active_path.exists());
    }
}

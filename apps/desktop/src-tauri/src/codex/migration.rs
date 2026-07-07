use crate::codex::backup::create_backup_dir;
use crate::codex::metadata::{metadata_from_bytes, replace_provider_marker, SessionMetadata};
use crate::codex::scan::{index_ids, session_files};
use crate::codex::sqlite::{state_dbs, upsert_state_entries};
use crate::codex::{
    CommandError, MigrationRequest, MigrationResult, PlannedRepair, Result, ThreadLifecycle,
};
use serde_json::json;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

pub fn preview_provider_migration(request: MigrationRequest) -> Result<MigrationResult> {
    migrate_provider(request, false)
}

pub fn apply_provider_migration(request: MigrationRequest) -> Result<MigrationResult> {
    migrate_provider(request, true)
}

fn migrate_provider(request: MigrationRequest, apply: bool) -> Result<MigrationResult> {
    let target_provider = request.target_provider.trim().to_string();
    if target_provider.is_empty() {
        return Err(CommandError::new(
            "target_provider_required",
            "Target provider is required.",
        ));
    }
    if request.thread_ids.is_empty() {
        return Err(CommandError::new(
            "no_session_selected",
            "Select at least one session to migrate.",
        ));
    }
    let codex_home = PathBuf::from(&request.codex_home);
    if !codex_home.exists() {
        return Err(CommandError::new(
            "codex_home_missing",
            format!("Codex home does not exist: {}", codex_home.display()),
        ));
    }
    let sessions_dir = codex_home.join("sessions");
    if !sessions_dir.exists() {
        return Err(CommandError::new(
            "sessions_missing",
            format!(
                "sessions directory does not exist: {}",
                sessions_dir.display()
            ),
        ));
    }
    let selected: BTreeSet<String> = request.thread_ids.into_iter().collect();
    let source_provider = request
        .source_provider
        .as_deref()
        .map(str::trim)
        .filter(|provider| !provider.is_empty())
        .map(str::to_string);
    let mut found_selected = BTreeSet::new();
    let mut changed = Vec::new();
    let mut metadata_items = Vec::new();
    let mut fixed_files = Vec::new();
    let title_overrides = title_overrides(&codex_home)?;

    for file in session_files(&codex_home)? {
        let raw = fs::read(&file.path)
            .map_err(|error| CommandError::io("read session", file.path.display(), error))?;
        let metadata = metadata_from_bytes(&raw, &file.path)?;
        if !selected.contains(&metadata.thread_id) {
            continue;
        }
        found_selected.insert(metadata.thread_id.clone());
        if let Some(source_provider) = source_provider.as_deref() {
            if metadata.provider.as_deref() != Some(source_provider) {
                continue;
            }
        }
        if metadata.provider.as_deref() == Some(target_provider.as_str()) {
            continue;
        }
        let fixed = replace_provider_marker(&raw, &target_provider)?;
        let mut fixed_metadata = metadata_from_bytes(&fixed, &file.path)?;
        if let Some(title) = title_overrides.get(&fixed_metadata.thread_id) {
            fixed_metadata.title = title.clone();
        }
        changed.push(fixed_metadata.thread_id.clone());
        metadata_items.push((fixed_metadata, file.lifecycle));
        fixed_files.push((file.path, fixed));
    }

    if let Some(missing) = selected.difference(&found_selected).next() {
        return Err(CommandError::new(
            "selected_thread_missing",
            format!("Selected thread no longer exists: {missing}"),
        ));
    }

    let planned_repairs = planned_repairs(&metadata_items);
    if !apply {
        return Ok(MigrationResult {
            changed_threads: changed,
            planned_repairs,
            backup_dir: None,
            dry_run: true,
        });
    }

    if fixed_files.is_empty() {
        return Ok(MigrationResult {
            changed_threads: changed,
            planned_repairs,
            backup_dir: None,
            dry_run: false,
        });
    }

    let backup_inputs = backup_inputs(&codex_home, &fixed_files);
    let backup_dir = create_backup_dir(&codex_home, &backup_inputs)?;
    for (path, fixed) in &fixed_files {
        fs::write(path, fixed)
            .map_err(|error| CommandError::io("write session", path.display(), error))
            .map_err(|error| {
                CommandError::post_backup(backup_dir.display(), "write session", error)
            })?;
    }
    for item in &metadata_items {
        ensure_index(&codex_home, item).map_err(|error| {
            CommandError::post_backup(backup_dir.display(), "update session index", error)
        })?;
    }
    upsert_state_entries(&codex_home, &metadata_items, &target_provider).map_err(|error| {
        CommandError::post_backup(
            backup_dir.display(),
            "update sqlite visibility metadata",
            error,
        )
    })?;

    Ok(MigrationResult {
        changed_threads: changed,
        planned_repairs,
        backup_dir: Some(backup_dir.display().to_string()),
        dry_run: false,
    })
}

fn planned_repairs(items: &[(SessionMetadata, ThreadLifecycle)]) -> Vec<PlannedRepair> {
    items
        .iter()
        .flat_map(|(metadata, _)| {
            [
                PlannedRepair {
                    thread_id: metadata.thread_id.clone(),
                    code: "update_provider".to_string(),
                    message: "更新会话文件中的 model_provider".to_string(),
                },
                PlannedRepair {
                    thread_id: metadata.thread_id.clone(),
                    code: "repair_visibility_metadata".to_string(),
                    message: "补齐索引和 sqlite 可见性元数据".to_string(),
                },
            ]
        })
        .collect()
}

fn backup_inputs(codex_home: &Path, fixed_files: &[(PathBuf, Vec<u8>)]) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = fixed_files.iter().map(|(path, _)| path.clone()).collect();
    let index = codex_home.join("session_index.jsonl");
    if index.exists() {
        files.push(index);
    }
    files.extend(state_dbs(codex_home));
    files
}

fn title_overrides(codex_home: &Path) -> Result<BTreeMap<String, String>> {
    let mut titles = sqlite_titles(codex_home)?;
    titles.extend(index_titles(codex_home)?);
    Ok(titles)
}

fn index_titles(codex_home: &Path) -> Result<BTreeMap<String, String>> {
    let path = codex_home.join("session_index.jsonl");
    if !path.exists() {
        return Ok(BTreeMap::new());
    }
    let text = fs::read_to_string(&path)
        .map_err(|error| CommandError::io("read session index", path.display(), error))?;
    let mut titles = BTreeMap::new();
    for line in text.lines().filter(|line| !line.trim().is_empty()) {
        let Ok(value) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        let Some(id) = value.get("id").and_then(Value::as_str) else {
            continue;
        };
        let Some(title) = value
            .get("thread_name")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
        else {
            continue;
        };
        titles.insert(id.to_string(), title.to_string());
    }
    Ok(titles)
}

fn sqlite_titles(codex_home: &Path) -> Result<BTreeMap<String, String>> {
    let mut titles = BTreeMap::new();
    for db in state_dbs(codex_home) {
        let Ok(connection) =
            rusqlite::Connection::open_with_flags(&db, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
        else {
            continue;
        };
        let Ok(mut statement) = connection
            .prepare("select id, title from threads where title is not null and trim(title) != ''")
        else {
            continue;
        };
        let Ok(rows) = statement.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        }) else {
            continue;
        };
        for row in rows {
            if let Ok((id, title)) = row {
                titles.entry(id).or_insert(title);
            }
        }
    }
    Ok(titles)
}

fn ensure_index(codex_home: &Path, item: &(SessionMetadata, ThreadLifecycle)) -> Result<()> {
    if item.1 == ThreadLifecycle::Archived {
        return Ok(());
    }
    let metadata = &item.0;
    let index_path = codex_home.join("session_index.jsonl");
    let existing = index_ids(codex_home)?;
    if existing.contains(&metadata.thread_id) {
        return Ok(());
    }
    if let Some(parent) = index_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| CommandError::io("create index parent", parent.display(), error))?;
    }
    let entry = json!({
        "id": metadata.thread_id,
        "thread_name": metadata.title,
        "updated_at": iso_from_ms(metadata.updated_at_ms),
    });
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&index_path)
        .map_err(|error| CommandError::io("open session index", index_path.display(), error))?;
    writeln!(file, "{}", serde_json::to_string(&entry).unwrap())
        .map_err(|error| CommandError::io("write session index", index_path.display(), error))?;
    Ok(())
}

fn iso_from_ms(value: i64) -> String {
    chrono::DateTime::from_timestamp_millis(value)
        .unwrap_or_else(chrono::Utc::now)
        .to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codex::test_support::{
        init_state_db, insert_state_row, insert_state_row_with_title, write_jsonl,
    };
    use rusqlite::Connection;

    #[test]
    fn preview_returns_changed_threads_without_writes() {
        let temp = tempfile::tempdir().unwrap();
        let codex = temp.path().join(".codex");
        let thread_id = "019eca3b-941d-7340-9b14-328c635a6523";
        let rollout =
            codex.join("sessions/2026/06/15/rollout-a-019eca3b-941d-7340-9b14-328c635a6523.jsonl");
        write_jsonl(&rollout, thread_id, "funai", false, "你好");
        let before = fs::read(&rollout).unwrap();

        let result = preview_provider_migration(MigrationRequest {
            codex_home: codex.display().to_string(),
            source_provider: Some("funai".to_string()),
            target_provider: "yihubangg".to_string(),
            thread_ids: vec![thread_id.to_string()],
        })
        .unwrap();

        assert_eq!(result.changed_threads, vec![thread_id.to_string()]);
        assert!(result.dry_run);
        assert_eq!(
            result
                .planned_repairs
                .iter()
                .map(|repair| repair.message.as_str())
                .collect::<Vec<_>>(),
            vec![
                "更新会话文件中的 model_provider",
                "补齐索引和 sqlite 可见性元数据"
            ]
        );
        assert_eq!(fs::read(&rollout).unwrap(), before);
    }

    #[test]
    fn apply_changes_provider_creates_backup_and_repairs_visibility() {
        let temp = tempfile::tempdir().unwrap();
        let codex = temp.path().join(".codex");
        let thread_id = "019ec94d-720d-7a12-a379-28c8042bc6b4";
        let rollout =
            codex.join("sessions/2026/06/15/rollout-a-019ec94d-720d-7a12-a379-28c8042bc6b4.jsonl");
        write_jsonl(&rollout, thread_id, "funai", true, "你好，保留中文");
        fs::write(codex.join("session_index.jsonl"), "").unwrap();
        let db = codex.join("sqlite/state_5.sqlite");
        init_state_db(&db);

        let result = apply_provider_migration(MigrationRequest {
            codex_home: codex.display().to_string(),
            source_provider: Some("funai".to_string()),
            target_provider: "yihubangg".to_string(),
            thread_ids: vec![thread_id.to_string()],
        })
        .unwrap();

        assert_eq!(result.changed_threads, vec![thread_id.to_string()]);
        assert!(!result.dry_run);
        let backup_dir = result.backup_dir.unwrap();
        assert!(Path::new(&backup_dir).exists());
        let raw = fs::read(&rollout).unwrap();
        assert!(!raw.starts_with(crate::codex::metadata::BOM));
        let text = String::from_utf8(raw).unwrap();
        assert!(text.contains("\"model_provider\":\"yihubangg\""));
        assert!(text.contains("你好，保留中文"));
        assert!(fs::read_to_string(codex.join("session_index.jsonl"))
            .unwrap()
            .contains(thread_id));
        let connection = Connection::open(&db).unwrap();
        let row: (String, i32, String) = connection
            .query_row(
                "select model_provider, archived, preview from threads where id=?1",
                [thread_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        assert_eq!(row.0, "yihubangg");
        assert_eq!(row.1, 0);
        assert!(row.2.contains("你好"));
    }

    #[test]
    fn apply_preserves_user_renamed_title_from_existing_state() {
        let temp = tempfile::tempdir().unwrap();
        let codex = temp.path().join(".codex");
        let thread_id = "019eee11-9343-7f30-971e-b01e55a058c8";
        let rollout =
            codex.join("sessions/2026/06/22/rollout-a-019eee11-9343-7f30-971e-b01e55a058c8.jsonl");
        write_jsonl(&rollout, thread_id, "funai", false, "old generated title");
        fs::write(
            codex.join("session_index.jsonl"),
            format!("{{\"id\":\"{thread_id}\",\"thread_name\":\"renamed title\"}}\n"),
        )
        .unwrap();
        let db = codex.join("state_5.sqlite");
        init_state_db(&db);
        insert_state_row_with_title(&db, thread_id, "funai", 0, "renamed title");

        apply_provider_migration(MigrationRequest {
            codex_home: codex.display().to_string(),
            source_provider: Some("funai".to_string()),
            target_provider: "yihubangg".to_string(),
            thread_ids: vec![thread_id.to_string()],
        })
        .unwrap();

        let connection = Connection::open(&db).unwrap();
        let title: String = connection
            .query_row(
                "select title from threads where id=?1",
                [thread_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(title, "renamed title");
    }

    #[test]
    fn apply_respects_selected_thread_ids() {
        let temp = tempfile::tempdir().unwrap();
        let codex = temp.path().join(".codex");
        let selected = "019eca3b-941d-7340-9b14-328c635a6523";
        let unselected = "019ec94d-720d-7a12-a379-28c8042bc6b4";
        let selected_rollout =
            codex.join("sessions/2026/06/15/rollout-a-019eca3b-941d-7340-9b14-328c635a6523.jsonl");
        let unselected_rollout =
            codex.join("sessions/2026/06/15/rollout-b-019ec94d-720d-7a12-a379-28c8042bc6b4.jsonl");
        write_jsonl(&selected_rollout, selected, "funai", false, "选中的会话");
        write_jsonl(
            &unselected_rollout,
            unselected,
            "funai",
            false,
            "未选中的会话",
        );
        let unselected_before = fs::read(&unselected_rollout).unwrap();

        let result = apply_provider_migration(MigrationRequest {
            codex_home: codex.display().to_string(),
            source_provider: Some("funai".to_string()),
            target_provider: "yihubangg".to_string(),
            thread_ids: vec![selected.to_string()],
        })
        .unwrap();

        assert_eq!(result.changed_threads, vec![selected.to_string()]);
        assert!(String::from_utf8(fs::read(&selected_rollout).unwrap())
            .unwrap()
            .contains("\"model_provider\":\"yihubangg\""));
        assert_eq!(fs::read(&unselected_rollout).unwrap(), unselected_before);
    }

    #[test]
    fn apply_supports_custom_target_provider() {
        let temp = tempfile::tempdir().unwrap();
        let codex = temp.path().join(".codex");
        let thread_id = "019eca3b-941d-7340-9b14-328c635a6523";
        let rollout =
            codex.join("sessions/2026/06/15/rollout-a-019eca3b-941d-7340-9b14-328c635a6523.jsonl");
        write_jsonl(&rollout, thread_id, "funai", false, "迁移到自定义 provider");

        let result = apply_provider_migration(MigrationRequest {
            codex_home: codex.display().to_string(),
            source_provider: Some("funai".to_string()),
            target_provider: "custom-provider".to_string(),
            thread_ids: vec![thread_id.to_string()],
        })
        .unwrap();

        assert_eq!(result.changed_threads, vec![thread_id.to_string()]);
        assert!(String::from_utf8(fs::read(&rollout).unwrap())
            .unwrap()
            .contains("\"model_provider\":\"custom-provider\""));
    }

    #[test]
    fn apply_repairs_both_state_database_locations_when_present() {
        let temp = tempfile::tempdir().unwrap();
        let codex = temp.path().join(".codex");
        let thread_id = "019eca3b-941d-7340-9b14-328c635a6523";
        let rollout =
            codex.join("sessions/2026/06/15/rollout-a-019eca3b-941d-7340-9b14-328c635a6523.jsonl");
        write_jsonl(&rollout, thread_id, "funai", false, "两个 sqlite 都要更新");
        let root_db = codex.join("state_5.sqlite");
        let nested_db = codex.join("sqlite/state_5.sqlite");
        init_state_db(&root_db);
        init_state_db(&nested_db);
        insert_state_row(&root_db, thread_id, "funai", 1);
        insert_state_row(&nested_db, thread_id, "funai", 1);

        apply_provider_migration(MigrationRequest {
            codex_home: codex.display().to_string(),
            source_provider: Some("funai".to_string()),
            target_provider: "yihubangg".to_string(),
            thread_ids: vec![thread_id.to_string()],
        })
        .unwrap();

        for db in [root_db, nested_db] {
            let connection = Connection::open(&db).unwrap();
            let row: (String, i32) = connection
                .query_row(
                    "select model_provider, archived from threads where id=?1",
                    [thread_id],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .unwrap();
            assert_eq!(row, ("yihubangg".to_string(), 0));
        }
    }

    #[test]
    fn apply_preserves_archived_state_for_archived_session_files() {
        let temp = tempfile::tempdir().unwrap();
        let codex = temp.path().join(".codex");
        fs::create_dir_all(codex.join("sessions")).unwrap();
        let thread_id = "019eca3b-941d-7340-9b14-328c635a6523";
        let rollout =
            codex.join("archived_sessions/rollout-a-019eca3b-941d-7340-9b14-328c635a6523.jsonl");
        write_jsonl(
            &rollout,
            thread_id,
            "funai",
            false,
            "archived provider migration",
        );
        let db = codex.join("state_5.sqlite");
        init_state_db(&db);
        insert_state_row(&db, thread_id, "funai", 1);

        let result = apply_provider_migration(MigrationRequest {
            codex_home: codex.display().to_string(),
            source_provider: Some("funai".to_string()),
            target_provider: "yihubangg".to_string(),
            thread_ids: vec![thread_id.to_string()],
        })
        .unwrap();

        assert_eq!(result.changed_threads, vec![thread_id.to_string()]);
        assert!(String::from_utf8(fs::read(&rollout).unwrap())
            .unwrap()
            .contains("\"model_provider\":\"yihubangg\""));
        let connection = Connection::open(&db).unwrap();
        let row: (String, i32) = connection
            .query_row(
                "select model_provider, archived from threads where id=?1",
                [thread_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(row, ("yihubangg".to_string(), 1));
    }

    #[test]
    fn preview_reports_missing_selected_thread() {
        let temp = tempfile::tempdir().unwrap();
        let codex = temp.path().join(".codex");
        write_jsonl(
            &codex.join("sessions/2026/06/15/rollout-a-019eca3b-941d-7340-9b14-328c635a6523.jsonl"),
            "019eca3b-941d-7340-9b14-328c635a6523",
            "funai",
            false,
            "你好",
        );

        let error = preview_provider_migration(MigrationRequest {
            codex_home: codex.display().to_string(),
            source_provider: Some("funai".to_string()),
            target_provider: "yihubangg".to_string(),
            thread_ids: vec!["019ec94d-720d-7a12-a379-28c8042bc6b4".to_string()],
        })
        .unwrap_err();

        assert_eq!(error.code, "selected_thread_missing");
    }

    #[test]
    fn preview_reports_missing_codex_home_before_sessions_directory() {
        let temp = tempfile::tempdir().unwrap();
        let codex = temp.path().join(".codex-missing");

        let error = preview_provider_migration(MigrationRequest {
            codex_home: codex.display().to_string(),
            source_provider: None,
            target_provider: "yihubangg".to_string(),
            thread_ids: vec!["019eca3b-941d-7340-9b14-328c635a6523".to_string()],
        })
        .unwrap_err();

        assert_eq!(error.code, "codex_home_missing");
    }

    #[test]
    fn preview_trims_source_provider_before_matching() {
        let temp = tempfile::tempdir().unwrap();
        let codex = temp.path().join(".codex");
        let thread_id = "019eca3b-941d-7340-9b14-328c635a6523";
        write_jsonl(
            &codex.join("sessions/2026/06/15/rollout-a-019eca3b-941d-7340-9b14-328c635a6523.jsonl"),
            thread_id,
            "funai",
            false,
            "你好",
        );

        let result = preview_provider_migration(MigrationRequest {
            codex_home: codex.display().to_string(),
            source_provider: Some(" funai ".to_string()),
            target_provider: "yihubangg".to_string(),
            thread_ids: vec![thread_id.to_string()],
        })
        .unwrap();

        assert_eq!(result.changed_threads, vec![thread_id.to_string()]);
    }

    #[test]
    fn apply_skips_backup_when_selected_threads_already_match_target_provider() {
        let temp = tempfile::tempdir().unwrap();
        let codex = temp.path().join(".codex");
        let thread_id = "019eca3b-941d-7340-9b14-328c635a6523";
        write_jsonl(
            &codex.join("sessions/2026/06/15/rollout-a-019eca3b-941d-7340-9b14-328c635a6523.jsonl"),
            thread_id,
            "yihubangg",
            false,
            "已经是目标 provider",
        );

        let result = apply_provider_migration(MigrationRequest {
            codex_home: codex.display().to_string(),
            source_provider: None,
            target_provider: "yihubangg".to_string(),
            thread_ids: vec![thread_id.to_string()],
        })
        .unwrap();

        assert!(result.changed_threads.is_empty());
        assert!(result.planned_repairs.is_empty());
        assert!(result.backup_dir.is_none());
        assert!(!result.dry_run);
        assert!(!codex.read_dir().unwrap().any(|entry| entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with("ai-session-migrator-backup-")));
    }

    #[test]
    fn apply_error_after_backup_includes_backup_dir_and_operation() {
        let temp = tempfile::tempdir().unwrap();
        let codex = temp.path().join(".codex");
        let thread_id = "019ec94d-720d-7a12-a379-28c8042bc6b4";
        let rollout =
            codex.join("sessions/2026/06/15/rollout-a-019ec94d-720d-7a12-a379-28c8042bc6b4.jsonl");
        write_jsonl(&rollout, thread_id, "funai", false, "你好");
        fs::write(codex.join("state_5.sqlite"), "not a sqlite database").unwrap();

        let error = apply_provider_migration(MigrationRequest {
            codex_home: codex.display().to_string(),
            source_provider: Some("funai".to_string()),
            target_provider: "yihubangg".to_string(),
            thread_ids: vec![thread_id.to_string()],
        })
        .unwrap_err();

        assert_eq!(error.code, "post_backup_write_failed");
        assert_eq!(
            error.operation.as_deref(),
            Some("update sqlite visibility metadata")
        );
        assert!(error.backup_dir.is_some());
        assert!(Path::new(error.backup_dir.as_deref().unwrap()).exists());
    }
}

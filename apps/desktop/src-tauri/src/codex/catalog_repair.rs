use crate::codex::backup::create_backup_dir;
use crate::codex::metadata::{metadata_from_bytes, SessionMetadata};
use crate::codex::paths::{normalize_windows_extended_path, visible_path_string};
use crate::codex::scan::session_files;
use crate::codex::sqlite::{state_dbs, state_entry, upsert_state_entries_from_metadata};
use crate::codex::{CommandError, Result, ThreadLifecycle};
use rusqlite::{params, Connection, OpenFlags};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

const ROLLOUT_INTERNAL_METADATA_CODE: &str = "rollout_internal_metadata_passthrough";
const ROLLOUT_INTERNAL_METADATA_KEY: &str = "internal_chat_message_metadata_passthrough";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CatalogRepairRow {
    pub thread_id: String,
    pub display_title: String,
    pub lifecycle: ThreadLifecycle,
    pub project_name: Option<String>,
    pub project_path: Option<String>,
    pub path: String,
    pub file_provider: Option<String>,
    pub repair_codes: Vec<String>,
    pub selected_by_default: bool,
    pub updated_at_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CatalogRepairSummary {
    pub total_threads: usize,
    pub missing_catalog_entries: usize,
    pub missing_session_index_entries: usize,
    pub state_metadata_issues: usize,
    pub selected_by_default: usize,
    pub archived_threads: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CatalogRepairScanResponse {
    pub rows: Vec<CatalogRepairRow>,
    pub summary: CatalogRepairSummary,
    pub catalog_db_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CatalogRepairRequest {
    pub codex_home: String,
    pub thread_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CatalogRepairChange {
    pub thread_id: String,
    pub action: String,
    pub display_title: String,
    pub cwd: String,
    pub source_kind: String,
    pub model_provider: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CatalogRepairResult {
    pub changed_threads: Vec<String>,
    pub planned_changes: Vec<CatalogRepairChange>,
    pub backup_dir: Option<String>,
    pub dry_run: bool,
}

#[derive(Debug, Clone)]
struct CatalogEntry {
    display_title: Option<String>,
    cwd: Option<String>,
}

#[derive(Debug, Clone)]
struct CatalogRepairCandidate {
    metadata: SessionMetadata,
    lifecycle: ThreadLifecycle,
    display_title: String,
    source_created_at: f64,
    source_updated_at: f64,
    repair_codes: Vec<String>,
}

pub fn scan_codex_catalog_repair(codex_home: &Path) -> Result<CatalogRepairScanResponse> {
    validate_codex_home(codex_home)?;

    let index_titles = index_titles(codex_home)?;
    let index_ids = session_index_ids(codex_home)?;
    let catalog_db = codex_home.join("sqlite/codex-dev.db");
    let catalog_entries = read_catalog_entries(&catalog_db)?;
    let catalog_db_path = catalog_db
        .exists()
        .then(|| catalog_db.display().to_string());
    let state_db_paths = state_dbs(codex_home);
    let files = session_files(codex_home)?;
    if files.is_empty() {
        return Err(CommandError::new(
            "no_sessions_found",
            format!(
                "No Codex sessions found in {}",
                codex_home.join("sessions").display()
            ),
        ));
    }

    let mut rows = Vec::new();
    for file in files {
        let raw = fs::read(&file.path)
            .map_err(|error| CommandError::io("read session", file.path.display(), error))?;
        let metadata = metadata_from_bytes(&raw, &file.path)?;
        let has_rollout_internal_passthrough =
            rollout_has_internal_metadata_passthrough(&raw, &file.path)?;
        let state_title = first_state_title(codex_home, &metadata.thread_id);
        let display_title = display_title(&metadata, &index_titles, state_title.as_deref());
        let repair_codes = repair_codes_for_metadata(
            &metadata,
            &display_title,
            &catalog_db,
            &catalog_entries,
            &index_ids,
            &state_db_paths,
            &file.lifecycle,
            has_rollout_internal_passthrough,
        );
        let selected_by_default = catalog_db.exists()
            && file.lifecycle == ThreadLifecycle::Active
            && repair_codes.iter().any(|code| is_repairable_code(code));
        rows.push(CatalogRepairRow {
            thread_id: metadata.thread_id.clone(),
            display_title,
            lifecycle: file.lifecycle,
            project_name: project_name_from_cwd(&metadata.cwd),
            project_path: non_empty_project_path(&metadata.cwd),
            path: visible_path_string(&metadata.path),
            file_provider: metadata.provider,
            repair_codes,
            selected_by_default,
            updated_at_ms: metadata.updated_at_ms,
        });
    }

    rows.sort_by(|left, right| {
        lifecycle_rank(&left.lifecycle)
            .cmp(&lifecycle_rank(&right.lifecycle))
            .then_with(|| right.selected_by_default.cmp(&left.selected_by_default))
            .then_with(|| right.updated_at_ms.cmp(&left.updated_at_ms))
            .then_with(|| left.thread_id.cmp(&right.thread_id))
    });

    let summary = CatalogRepairSummary {
        total_threads: rows.len(),
        missing_catalog_entries: rows
            .iter()
            .filter(|row| {
                row.repair_codes
                    .iter()
                    .any(|code| code == "missing_catalog_entry")
            })
            .count(),
        missing_session_index_entries: rows
            .iter()
            .filter(|row| {
                row.repair_codes
                    .iter()
                    .any(|code| code == "missing_session_index")
            })
            .count(),
        state_metadata_issues: rows
            .iter()
            .filter(|row| {
                row.repair_codes.iter().any(|code| {
                    matches!(
                        code.as_str(),
                        "missing_state_entry"
                            | "state_rollout_path_mismatch"
                            | "state_cwd_mismatch"
                    )
                })
            })
            .count(),
        selected_by_default: rows.iter().filter(|row| row.selected_by_default).count(),
        archived_threads: rows
            .iter()
            .filter(|row| row.lifecycle == ThreadLifecycle::Archived)
            .count(),
    };

    Ok(CatalogRepairScanResponse {
        rows,
        summary,
        catalog_db_path,
    })
}

pub fn preview_codex_catalog_repair(request: CatalogRepairRequest) -> Result<CatalogRepairResult> {
    let codex_home = PathBuf::from(&request.codex_home);
    let candidates = selected_catalog_repair_candidates(&codex_home, &request.thread_ids)?;
    Ok(CatalogRepairResult {
        changed_threads: candidates
            .iter()
            .map(|candidate| candidate.metadata.thread_id.clone())
            .collect(),
        planned_changes: catalog_repair_changes(&candidates),
        backup_dir: None,
        dry_run: true,
    })
}

pub fn apply_codex_catalog_repair(request: CatalogRepairRequest) -> Result<CatalogRepairResult> {
    apply_codex_catalog_repair_with_process_checker(request, is_codex_running)
}

fn apply_codex_catalog_repair_with_process_checker(
    request: CatalogRepairRequest,
    is_codex_running: impl FnOnce() -> bool,
) -> Result<CatalogRepairResult> {
    if request.thread_ids.is_empty() {
        return Err(CommandError::new(
            "no_threads_selected",
            "Select at least one session to repair.",
        ));
    }
    if is_codex_running() {
        return Err(CommandError::new(
            "codex_process_running",
            "Close Codex Desktop and Codex CLI before repairing the visible index.",
        ));
    }
    let codex_home = PathBuf::from(&request.codex_home);
    let candidates = selected_catalog_repair_candidates(&codex_home, &request.thread_ids)?;
    let changed_threads: Vec<String> = candidates
        .iter()
        .map(|candidate| candidate.metadata.thread_id.clone())
        .collect();
    let planned_changes = catalog_repair_changes(&candidates);
    if candidates.is_empty() {
        return Ok(CatalogRepairResult {
            changed_threads,
            planned_changes,
            backup_dir: None,
            dry_run: false,
        });
    }

    let backup_dir = create_backup_dir(&codex_home, &backup_inputs(&codex_home, &candidates))?;
    strip_rollout_internal_metadata_passthrough_for_candidates(&candidates).map_err(|error| {
        CommandError::post_backup(
            backup_dir.display(),
            "repair rollout compatibility metadata",
            error,
        )
    })?;
    write_catalog_entries(&codex_home, &candidates).map_err(|error| {
        CommandError::post_backup(backup_dir.display(), "update codex catalog", error)
    })?;
    ensure_session_index_entries(&codex_home, &candidates).map_err(|error| {
        CommandError::post_backup(backup_dir.display(), "update session index", error)
    })?;
    let state_items = state_repair_items(&candidates);
    if !state_items.is_empty() {
        upsert_state_entries_from_metadata(&codex_home, &state_items).map_err(|error| {
            CommandError::post_backup(
                backup_dir.display(),
                "update sqlite visibility metadata",
                error,
            )
        })?;
    }

    Ok(CatalogRepairResult {
        changed_threads,
        planned_changes,
        backup_dir: Some(backup_dir.display().to_string()),
        dry_run: false,
    })
}

fn selected_catalog_repair_candidates(
    codex_home: &Path,
    thread_ids: &[String],
) -> Result<Vec<CatalogRepairCandidate>> {
    if thread_ids.is_empty() {
        return Err(CommandError::new(
            "no_threads_selected",
            "Select at least one session to repair.",
        ));
    }
    validate_codex_home(codex_home)?;
    let catalog_db = codex_home.join("sqlite/codex-dev.db");
    if !catalog_db.exists() {
        return Err(CommandError::new(
            "catalog_db_missing",
            format!(
                "Codex catalog database does not exist: {}",
                catalog_db.display()
            ),
        ));
    }
    let selected: std::collections::BTreeSet<&str> =
        thread_ids.iter().map(String::as_str).collect();
    let index_titles = index_titles(codex_home)?;
    let index_ids = session_index_ids(codex_home)?;
    let catalog_entries = read_catalog_entries(&catalog_db)?;
    let state_db_paths = state_dbs(codex_home);
    let mut found = std::collections::BTreeSet::new();
    let mut candidates = Vec::new();

    for file in session_files(codex_home)? {
        let raw = fs::read(&file.path)
            .map_err(|error| CommandError::io("read session", file.path.display(), error))?;
        let metadata = metadata_from_bytes(&raw, &file.path)?;
        if !selected.contains(metadata.thread_id.as_str()) {
            continue;
        }
        let has_rollout_internal_passthrough =
            rollout_has_internal_metadata_passthrough(&raw, &file.path)?;
        found.insert(metadata.thread_id.clone());
        let state_title = first_state_title(codex_home, &metadata.thread_id);
        let display_title = display_title(&metadata, &index_titles, state_title.as_deref());
        let repair_codes = repair_codes_for_metadata(
            &metadata,
            &display_title,
            &catalog_db,
            &catalog_entries,
            &index_ids,
            &state_db_paths,
            &file.lifecycle,
            has_rollout_internal_passthrough,
        );
        if !repair_codes.iter().any(|code| is_repairable_code(code)) {
            continue;
        }
        candidates.push(CatalogRepairCandidate {
            lifecycle: file.lifecycle,
            source_created_at: seconds_from_ms(metadata.created_at_ms),
            source_updated_at: seconds_from_ms(metadata.updated_at_ms),
            metadata,
            display_title,
            repair_codes,
        });
    }

    for thread_id in thread_ids {
        if !found.contains(thread_id) {
            return Err(CommandError::new(
                "selected_thread_missing",
                format!("Selected thread no longer exists: {thread_id}"),
            ));
        }
    }
    Ok(candidates)
}

fn catalog_repair_changes(candidates: &[CatalogRepairCandidate]) -> Vec<CatalogRepairChange> {
    candidates
        .iter()
        .flat_map(|candidate| {
            let mut actions = Vec::new();
            if candidate
                .repair_codes
                .iter()
                .any(|code| code == "missing_catalog_entry")
            {
                actions.push("insert_catalog_entry");
            }
            if candidate.repair_codes.iter().any(|code| {
                matches!(
                    code.as_str(),
                    "catalog_cwd_mismatch" | "catalog_title_stale"
                )
            }) {
                actions.push("update_catalog_entry");
            }
            if candidate
                .repair_codes
                .iter()
                .any(|code| code == "missing_session_index")
            {
                actions.push("append_session_index_entry");
            }
            if candidate
                .repair_codes
                .iter()
                .any(|code| code == "missing_state_entry")
            {
                actions.push("upsert_state_entry");
            }
            if candidate.repair_codes.iter().any(|code| {
                matches!(
                    code.as_str(),
                    "state_rollout_path_mismatch" | "state_cwd_mismatch"
                )
            }) && !actions.contains(&"upsert_state_entry")
            {
                actions.push("upsert_state_entry");
            }
            if candidate
                .repair_codes
                .iter()
                .any(|code| code == ROLLOUT_INTERNAL_METADATA_CODE)
            {
                actions.push("strip_rollout_internal_metadata_passthrough");
            }
            actions.into_iter().map(|action| CatalogRepairChange {
                thread_id: candidate.metadata.thread_id.clone(),
                action: action.to_string(),
                display_title: candidate.display_title.clone(),
                cwd: normalize_windows_extended_path(&candidate.metadata.cwd),
                source_kind: candidate.metadata.source.clone(),
                model_provider: candidate.metadata.provider.clone(),
            })
        })
        .collect()
}

fn write_catalog_entries(codex_home: &Path, candidates: &[CatalogRepairCandidate]) -> Result<()> {
    let catalog_db = codex_home.join("sqlite/codex-dev.db");
    let mut connection = Connection::open(&catalog_db)
        .map_err(|error| CommandError::new("sqlite_open_failed", error.to_string()))?;
    connection
        .busy_timeout(Duration::from_millis(5_000))
        .map_err(|error| CommandError::new("sqlite_busy_timeout_failed", error.to_string()))?;
    ensure_catalog_schema(&connection)?;
    let transaction = connection
        .transaction()
        .map_err(|error| CommandError::new("sqlite_transaction_failed", error.to_string()))?;
    transaction
        .execute(
            "insert or ignore into local_thread_catalog_hosts (host_id, host_kind) values ('local', 'local')",
            [],
        )
        .map_err(|error| CommandError::new("catalog_insert_failed", error.to_string()))?;
    let mut observation_sequence: i64 = transaction
        .query_row(
            "select coalesce(max(observation_sequence), 0) from local_thread_catalog",
            [],
            |row| row.get(0),
        )
        .map_err(|error| CommandError::new("sqlite_query_failed", error.to_string()))?;
    for candidate in candidates {
        let cwd = normalize_windows_extended_path(&candidate.metadata.cwd);
        if candidate
            .repair_codes
            .iter()
            .any(|code| code == "missing_catalog_entry")
        {
            observation_sequence += 1;
            transaction
                .execute(
                    "
                    insert into local_thread_catalog (
                        host_id, thread_id, display_title, source_created_at, source_updated_at,
                        cwd, source_kind, source_detail, model_provider, git_branch,
                        observation_sequence, missing_candidate
                    ) values (
                        'local', ?1, ?2, ?3, ?4, ?5, ?6, '', ?7, NULL, ?8, 0
                    )
                    ",
                    params![
                        &candidate.metadata.thread_id,
                        &candidate.display_title,
                        candidate.source_created_at,
                        candidate.source_updated_at,
                        &cwd,
                        &candidate.metadata.source,
                        &candidate.metadata.provider,
                        observation_sequence,
                    ],
                )
                .map_err(|error| CommandError::new("catalog_insert_failed", error.to_string()))?;
        } else if candidate.repair_codes.iter().any(|code| {
            matches!(
                code.as_str(),
                "catalog_cwd_mismatch" | "catalog_title_stale"
            )
        }) {
            transaction
                .execute(
                    "
                    update local_thread_catalog
                    set display_title=?1, source_created_at=?2, source_updated_at=?3,
                        cwd=?4, source_kind=?5, source_detail='', model_provider=?6,
                        missing_candidate=0
                    where host_id='local' and thread_id=?7
                    ",
                    params![
                        &candidate.display_title,
                        candidate.source_created_at,
                        candidate.source_updated_at,
                        &cwd,
                        &candidate.metadata.source,
                        &candidate.metadata.provider,
                        &candidate.metadata.thread_id,
                    ],
                )
                .map_err(|error| CommandError::new("catalog_update_failed", error.to_string()))?;
        }
    }
    transaction
        .commit()
        .map_err(|error| CommandError::new("sqlite_transaction_failed", error.to_string()))?;
    Ok(())
}

fn repair_codes_for_metadata(
    metadata: &SessionMetadata,
    display_title: &str,
    catalog_db: &Path,
    catalog_entries: &BTreeMap<String, CatalogEntry>,
    index_ids: &BTreeSet<String>,
    state_db_paths: &[PathBuf],
    lifecycle: &ThreadLifecycle,
    has_rollout_internal_passthrough: bool,
) -> Vec<String> {
    let mut repair_codes = Vec::new();
    let expected_cwd = normalize_windows_extended_path(&metadata.cwd);
    if !catalog_db.exists() {
        repair_codes.push("catalog_db_missing".to_string());
    } else if let Some(entry) = catalog_entries.get(&metadata.thread_id) {
        if entry.cwd.as_deref().unwrap_or_default() != expected_cwd {
            repair_codes.push("catalog_cwd_mismatch".to_string());
        }
        if entry
            .display_title
            .as_deref()
            .unwrap_or_default()
            .trim()
            .is_empty()
            && !display_title.trim().is_empty()
        {
            repair_codes.push("catalog_title_stale".to_string());
        }
    } else {
        repair_codes.push("missing_catalog_entry".to_string());
    }
    if lifecycle == &ThreadLifecycle::Active && !index_ids.contains(&metadata.thread_id) {
        repair_codes.push("missing_session_index".to_string());
    }
    let expected_rollout_path = visible_path_string(&metadata.path);
    for db in state_db_paths {
        let entry = state_entry(db, &metadata.thread_id);
        if !entry.exists {
            repair_codes.push("missing_state_entry".to_string());
            continue;
        }
        if entry.rollout_path.as_deref().unwrap_or_default() != expected_rollout_path {
            repair_codes.push("state_rollout_path_mismatch".to_string());
        }
        if entry.cwd.as_deref().unwrap_or_default() != expected_cwd {
            repair_codes.push("state_cwd_mismatch".to_string());
        }
    }
    if has_rollout_internal_passthrough {
        repair_codes.push(ROLLOUT_INTERNAL_METADATA_CODE.to_string());
    }
    repair_codes.sort();
    repair_codes.dedup();
    repair_codes
}

fn is_repairable_code(code: &str) -> bool {
    matches!(
        code,
        "missing_catalog_entry"
            | "catalog_cwd_mismatch"
            | "catalog_title_stale"
            | "missing_session_index"
            | "missing_state_entry"
            | "state_rollout_path_mismatch"
            | "state_cwd_mismatch"
            | ROLLOUT_INTERNAL_METADATA_CODE
    )
}

fn rollout_has_internal_metadata_passthrough(raw: &[u8], path: &Path) -> Result<bool> {
    let without_bom = if raw.starts_with(crate::codex::metadata::BOM) {
        &raw[crate::codex::metadata::BOM.len()..]
    } else {
        raw
    };
    let text = std::str::from_utf8(without_bom).map_err(|error| {
        CommandError::new(
            "invalid_utf8",
            format!("{} is not valid UTF-8 ({error})", path.display()),
        )
    })?;
    for line in text.lines().filter(|line| !line.trim().is_empty()) {
        let value: Value = serde_json::from_str(line).map_err(|error| {
            CommandError::new(
                "invalid_jsonl",
                format!("{} has invalid JSONL row ({error})", path.display()),
            )
        })?;
        if value
            .get("payload")
            .and_then(Value::as_object)
            .is_some_and(|payload| payload.contains_key(ROLLOUT_INTERNAL_METADATA_KEY))
        {
            return Ok(true);
        }
    }
    Ok(false)
}

fn strip_rollout_internal_metadata_passthrough_for_candidates(
    candidates: &[CatalogRepairCandidate],
) -> Result<()> {
    for candidate in candidates {
        if !candidate
            .repair_codes
            .iter()
            .any(|code| code == ROLLOUT_INTERNAL_METADATA_CODE)
        {
            continue;
        }
        let path = &candidate.metadata.path;
        let raw = fs::read(path)
            .map_err(|error| CommandError::io("read rollout", path.display(), error))?;
        let Some(repaired) = strip_rollout_internal_metadata_passthrough(&raw, path)? else {
            continue;
        };
        fs::write(path, repaired)
            .map_err(|error| CommandError::io("write rollout", path.display(), error))?;
    }
    Ok(())
}

fn strip_rollout_internal_metadata_passthrough(raw: &[u8], path: &Path) -> Result<Option<Vec<u8>>> {
    let has_bom = raw.starts_with(crate::codex::metadata::BOM);
    let without_bom = if has_bom {
        &raw[crate::codex::metadata::BOM.len()..]
    } else {
        raw
    };
    let text = std::str::from_utf8(without_bom).map_err(|error| {
        CommandError::new(
            "invalid_utf8",
            format!("{} is not valid UTF-8 ({error})", path.display()),
        )
    })?;
    let mut changed = false;
    let mut output = Vec::with_capacity(raw.len());
    if has_bom {
        output.extend_from_slice(crate::codex::metadata::BOM);
    }
    for line in text.split_inclusive('\n') {
        let (body, newline) = split_jsonl_line_suffix(line);
        if body.trim().is_empty() {
            output.extend_from_slice(line.as_bytes());
            continue;
        }
        let mut value: Value = serde_json::from_str(body).map_err(|error| {
            CommandError::new(
                "invalid_jsonl",
                format!("{} has invalid JSONL row ({error})", path.display()),
            )
        })?;
        let removed = value
            .get_mut("payload")
            .and_then(Value::as_object_mut)
            .and_then(|payload| payload.remove(ROLLOUT_INTERNAL_METADATA_KEY))
            .is_some();
        if removed {
            changed = true;
            let serialized = serde_json::to_string(&value).map_err(|error| {
                CommandError::new(
                    "invalid_jsonl",
                    format!(
                        "{} cannot serialize repaired JSONL row ({error})",
                        path.display()
                    ),
                )
            })?;
            output.extend_from_slice(serialized.as_bytes());
            output.extend_from_slice(newline.as_bytes());
        } else {
            output.extend_from_slice(line.as_bytes());
        }
    }
    Ok(changed.then_some(output))
}

fn split_jsonl_line_suffix(line: &str) -> (&str, &str) {
    if let Some(body) = line.strip_suffix("\r\n") {
        (body, "\r\n")
    } else if let Some(body) = line.strip_suffix('\n') {
        (body, "\n")
    } else {
        (line, "")
    }
}

fn ensure_session_index_entries(
    codex_home: &Path,
    candidates: &[CatalogRepairCandidate],
) -> Result<()> {
    let mut existing = session_index_ids(codex_home)?;
    let mut entries = Vec::new();
    for candidate in candidates {
        if candidate.lifecycle != ThreadLifecycle::Active
            || !candidate
                .repair_codes
                .iter()
                .any(|code| code == "missing_session_index")
            || !existing.insert(candidate.metadata.thread_id.clone())
        {
            continue;
        }
        entries.push(json!({
            "id": &candidate.metadata.thread_id,
            "thread_name": &candidate.display_title,
            "updated_at": iso_from_ms(candidate.metadata.updated_at_ms),
        }));
    }
    if entries.is_empty() {
        return Ok(());
    }
    let index_path = codex_home.join("session_index.jsonl");
    if let Some(parent) = index_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| CommandError::io("create index parent", parent.display(), error))?;
    }
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&index_path)
        .map_err(|error| CommandError::io("open session index", index_path.display(), error))?;
    for entry in entries {
        writeln!(file, "{}", serde_json::to_string(&entry).unwrap()).map_err(|error| {
            CommandError::io("write session index", index_path.display(), error)
        })?;
    }
    Ok(())
}

fn state_repair_items(
    candidates: &[CatalogRepairCandidate],
) -> Vec<(SessionMetadata, ThreadLifecycle)> {
    candidates
        .iter()
        .filter(|candidate| {
            candidate.repair_codes.iter().any(|code| {
                matches!(
                    code.as_str(),
                    "missing_state_entry" | "state_rollout_path_mismatch" | "state_cwd_mismatch"
                )
            })
        })
        .map(|candidate| {
            let mut metadata = candidate.metadata.clone();
            metadata.title = candidate.display_title.clone();
            (metadata, candidate.lifecycle.clone())
        })
        .collect()
}

fn iso_from_ms(value: i64) -> String {
    chrono::DateTime::from_timestamp_millis(value)
        .unwrap_or_else(chrono::Utc::now)
        .to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

fn backup_inputs(codex_home: &Path, candidates: &[CatalogRepairCandidate]) -> Vec<PathBuf> {
    let mut inputs: Vec<PathBuf> = [
        "sqlite/codex-dev.db",
        "sqlite/codex-dev.db-wal",
        "sqlite/codex-dev.db-shm",
        "state_5.sqlite",
        "state_5.sqlite-wal",
        "state_5.sqlite-shm",
        "sqlite/state_5.sqlite",
        "sqlite/state_5.sqlite-wal",
        "sqlite/state_5.sqlite-shm",
        "session_index.jsonl",
    ]
    .into_iter()
    .map(|relative| codex_home.join(relative))
    .collect();
    inputs.extend(
        candidates
            .iter()
            .filter(|candidate| {
                candidate
                    .repair_codes
                    .iter()
                    .any(|code| code == ROLLOUT_INTERNAL_METADATA_CODE)
            })
            .map(|candidate| candidate.metadata.path.clone()),
    );
    inputs
}

fn ensure_catalog_schema(connection: &Connection) -> Result<()> {
    if !has_table(connection, "local_thread_catalog")?
        || !has_table(connection, "local_thread_catalog_hosts")?
    {
        return Err(CommandError::new(
            "catalog_schema_unsupported",
            "codex-dev.db does not contain the expected catalog tables.",
        ));
    }
    let required_columns = [
        "host_id",
        "thread_id",
        "display_title",
        "source_created_at",
        "source_updated_at",
        "cwd",
        "source_kind",
        "source_detail",
        "model_provider",
        "git_branch",
        "observation_sequence",
        "missing_candidate",
    ];
    for column in required_columns {
        if !has_column(connection, "local_thread_catalog", column)? {
            return Err(CommandError::new(
                "catalog_schema_unsupported",
                format!("local_thread_catalog is missing required column {column}."),
            ));
        }
    }
    Ok(())
}

fn validate_codex_home(codex_home: &Path) -> Result<()> {
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
    Ok(())
}

fn read_catalog_entries(db_path: &Path) -> Result<BTreeMap<String, CatalogEntry>> {
    if !db_path.exists() {
        return Ok(BTreeMap::new());
    }
    let connection = Connection::open_with_flags(db_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|error| CommandError::new("sqlite_open_failed", error.to_string()))?;
    if !has_table(&connection, "local_thread_catalog")? {
        return Err(CommandError::new(
            "catalog_schema_unsupported",
            "codex-dev.db does not contain local_thread_catalog.",
        ));
    }
    let required_columns = [
        "thread_id",
        "display_title",
        "cwd",
        "model_provider",
        "observation_sequence",
        "missing_candidate",
    ];
    for column in required_columns {
        if !has_column(&connection, "local_thread_catalog", column)? {
            return Err(CommandError::new(
                "catalog_schema_unsupported",
                format!("local_thread_catalog is missing required column {column}."),
            ));
        }
    }
    let mut statement = connection
        .prepare("select thread_id, display_title, cwd from local_thread_catalog")
        .map_err(|error| CommandError::new("catalog_schema_unsupported", error.to_string()))?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                CatalogEntry {
                    display_title: row.get::<_, Option<String>>(1)?,
                    cwd: row.get::<_, Option<String>>(2)?,
                },
            ))
        })
        .map_err(|error| CommandError::new("sqlite_query_failed", error.to_string()))?;
    let mut entries = BTreeMap::new();
    for row in rows {
        let (thread_id, entry) =
            row.map_err(|error| CommandError::new("sqlite_query_failed", error.to_string()))?;
        entries.insert(thread_id, entry);
    }
    Ok(entries)
}

fn has_table(connection: &Connection, table: &str) -> Result<bool> {
    let count: i64 = connection
        .query_row(
            "select count(*) from sqlite_master where type='table' and name=?1",
            [table],
            |row| row.get(0),
        )
        .map_err(|error| CommandError::new("sqlite_query_failed", error.to_string()))?;
    Ok(count > 0)
}

fn has_column(connection: &Connection, table: &str, column: &str) -> Result<bool> {
    let mut statement = connection
        .prepare(&format!("pragma table_info({table})"))
        .map_err(|error| CommandError::new("sqlite_query_failed", error.to_string()))?;
    let rows = statement
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|error| CommandError::new("sqlite_query_failed", error.to_string()))?;
    for row in rows {
        if row.map_err(|error| CommandError::new("sqlite_query_failed", error.to_string()))?
            == column
        {
            return Ok(true);
        }
    }
    Ok(false)
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

fn session_index_ids(codex_home: &Path) -> Result<BTreeSet<String>> {
    let path = codex_home.join("session_index.jsonl");
    if !path.exists() {
        return Ok(BTreeSet::new());
    }
    let text = fs::read_to_string(&path)
        .map_err(|error| CommandError::io("read session index", path.display(), error))?;
    let mut ids = BTreeSet::new();
    for line in text.lines().filter(|line| !line.trim().is_empty()) {
        let Ok(value) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        if let Some(id) = value.get("id").and_then(Value::as_str) {
            ids.insert(id.to_string());
        }
    }
    Ok(ids)
}

fn first_state_title(codex_home: &Path, thread_id: &str) -> Option<String> {
    for db in state_dbs(codex_home) {
        let entry = state_entry(&db, thread_id);
        if entry.exists {
            if let Some(title) = entry.title {
                return Some(title);
            }
        }
    }
    None
}

fn display_title(
    metadata: &SessionMetadata,
    index_titles: &BTreeMap<String, String>,
    state_title: Option<&str>,
) -> String {
    index_titles
        .get(&metadata.thread_id)
        .filter(|title| !title.trim().is_empty())
        .cloned()
        .or_else(|| {
            state_title
                .map(str::trim)
                .filter(|title| !title.is_empty())
                .map(str::to_string)
        })
        .unwrap_or_else(|| metadata.title.clone())
}

fn project_name_from_cwd(cwd: &str) -> Option<String> {
    let normalized = normalize_windows_extended_path(cwd);
    normalized
        .trim()
        .trim_end_matches(['/', '\\'])
        .split(['/', '\\'])
        .filter(|part| !part.is_empty())
        .next_back()
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(str::to_string)
}

fn non_empty_project_path(cwd: &str) -> Option<String> {
    let normalized = normalize_windows_extended_path(cwd);
    let trimmed = normalized.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

fn lifecycle_rank(lifecycle: &ThreadLifecycle) -> u8 {
    match lifecycle {
        ThreadLifecycle::Active => 0,
        ThreadLifecycle::Archived => 1,
    }
}

fn seconds_from_ms(value: i64) -> f64 {
    value as f64 / 1000.0
}

fn is_codex_running() -> bool {
    running_process_names()
        .iter()
        .any(|name| is_codex_process_name(name))
}

fn running_process_names() -> Vec<String> {
    if cfg!(target_os = "windows") {
        return tasklist_process_names();
    }
    ps_process_names()
}

fn tasklist_process_names() -> Vec<String> {
    let Ok(output) = Command::new("tasklist")
        .args(["/FO", "CSV", "/NH"])
        .output()
    else {
        return Vec::new();
    };
    if !output.status.success() {
        return Vec::new();
    }
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(first_csv_field)
        .map(str::to_string)
        .collect()
}

fn ps_process_names() -> Vec<String> {
    let Ok(output) = Command::new("ps").args(["-axo", "comm="]).output() else {
        return Vec::new();
    };
    if !output.status.success() {
        return Vec::new();
    }
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(str::to_string)
        .collect()
}

fn first_csv_field(line: &str) -> Option<&str> {
    let line = line.trim();
    if !line.starts_with('"') {
        return line
            .split(',')
            .next()
            .map(str::trim)
            .filter(|value| !value.is_empty());
    }
    let rest = &line[1..];
    let end = rest.find('"')?;
    let value = &rest[..end];
    (!value.is_empty()).then_some(value)
}

fn is_codex_process_name(name: &str) -> bool {
    let normalized = name
        .trim()
        .trim_end_matches(".exe")
        .trim_end_matches(".EXE")
        .to_ascii_lowercase();
    matches!(
        normalized.as_str(),
        "codex" | "codex desktop" | "codex-desktop"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codex::test_support::{init_state_db, insert_state_row_with_title, write_jsonl};
    use rusqlite::Connection;
    use std::fs;

    fn init_catalog_db(path: &std::path::Path) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let connection = Connection::open(path).unwrap();
        connection
            .execute_batch(
                "
                create table local_thread_catalog_hosts (host_id text primary key, host_kind text);
                create table local_thread_catalog (
                    host_id text,
                    thread_id text,
                    display_title text,
                    source_created_at real,
                    source_updated_at real,
                    cwd text,
                    source_kind text,
                    source_detail text,
                    model_provider text,
                    git_branch text,
                    observation_sequence integer,
                    missing_candidate integer
                );
                create table local_thread_catalog_sync_state (
                    host_id text primary key,
                    watermark_updated_at real,
                    initial_build_complete integer,
                    observation_sequence integer
                );
                ",
            )
            .unwrap();
    }

    fn insert_catalog_row(path: &std::path::Path, thread_id: &str, display_title: &str) {
        let connection = Connection::open(path).unwrap();
        connection
            .execute(
                "
                insert into local_thread_catalog (
                    host_id, thread_id, display_title, source_created_at, source_updated_at,
                    cwd, source_kind, source_detail, model_provider, git_branch,
                    observation_sequence, missing_candidate
                ) values (
                    'local', ?1, ?2, 1781485200.0, 1781485260.0, 'D:\\work',
                    'vscode', '', 'funai', NULL, 1, 0
                )
                ",
                (thread_id, display_title),
            )
            .unwrap();
    }

    fn poison_state_paths(
        state_db: &std::path::Path,
        thread_id: &str,
        rollout_path: &std::path::Path,
    ) {
        let connection = Connection::open(state_db).unwrap();
        connection
            .execute(
                "update threads set rollout_path=?1, cwd=?2 where id=?3",
                (
                    format!(r"\\?\{}", rollout_path.display()),
                    r"\\?\D:\work",
                    thread_id,
                ),
            )
            .unwrap();
    }

    fn set_state_paths(
        state_db: &std::path::Path,
        thread_id: &str,
        rollout_path: &std::path::Path,
        cwd: &str,
    ) {
        let connection = Connection::open(state_db).unwrap();
        connection
            .execute(
                "update threads set rollout_path=?1, cwd=?2 where id=?3",
                (visible_path_string(rollout_path), cwd, thread_id),
            )
            .unwrap();
    }

    fn write_jsonl_with_internal_passthrough(path: &std::path::Path, thread_id: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(
            path,
            format!(
                "{{\"timestamp\":\"2026-07-05T15:31:44.000Z\",\"type\":\"session_meta\",\"payload\":{{\"id\":\"{thread_id}\",\"session_id\":\"{thread_id}\",\"timestamp\":\"2026-07-05T15:31:44.000Z\",\"cwd\":\"D:\\\\work\",\"source\":\"vscode\",\"model_provider\":\"funai\",\"cli_version\":\"0.142.5\",\"thread_source\":\"user\",\"base_instructions\":\"keep\",\"dynamic_tools\":[]}}}}\n\
                 {{\"timestamp\":\"2026-07-05T15:31:45.000Z\",\"type\":\"response_item\",\"payload\":{{\"type\":\"message\",\"role\":\"user\",\"content\":[{{\"type\":\"input_text\",\"text\":\"hello\"}}],\"internal_chat_message_metadata_passthrough\":{{\"turn_id\":\"turn-1\"}}}}}}\n"
            ),
        )
        .unwrap();
    }

    #[test]
    fn scan_reports_active_session_missing_from_catalog() {
        let temp = tempfile::tempdir().unwrap();
        let codex = temp.path().join(".codex");
        let thread_id = "019f32e8-178a-7b01-9a43-61e5a75d73ae";
        write_jsonl(
            &codex.join("sessions/2026/07/05/rollout-a-019f32e8-178a-7b01-9a43-61e5a75d73ae.jsonl"),
            thread_id,
            "funai",
            false,
            "json title",
        );
        fs::write(
            codex.join("session_index.jsonl"),
            format!("{{\"id\":\"{thread_id}\",\"thread_name\":\"renamed title\"}}\n"),
        )
        .unwrap();
        let state_db = codex.join("state_5.sqlite");
        init_state_db(&state_db);
        insert_state_row_with_title(&state_db, thread_id, "funai", 0, "sqlite title");
        let discovered_path = session_files(&codex).unwrap()[0].path.clone();
        set_state_paths(&state_db, thread_id, &discovered_path, "D:\\work");
        init_catalog_db(&codex.join("sqlite/codex-dev.db"));

        let response = scan_codex_catalog_repair(&codex).unwrap();

        assert_eq!(response.summary.total_threads, 1);
        assert_eq!(response.summary.missing_catalog_entries, 1);
        assert_eq!(response.rows[0].thread_id, thread_id);
        assert_eq!(response.rows[0].display_title, "renamed title");
        assert_eq!(response.rows[0].repair_codes, vec!["missing_catalog_entry"]);
        assert!(response.rows[0].selected_by_default);
    }

    #[test]
    fn preview_returns_insert_change_without_writing_catalog() {
        let temp = tempfile::tempdir().unwrap();
        let codex = temp.path().join(".codex");
        let thread_id = "019f32e8-178a-7b01-9a43-61e5a75d73ae";
        write_jsonl(
            &codex.join("sessions/2026/07/05/rollout-a-019f32e8-178a-7b01-9a43-61e5a75d73ae.jsonl"),
            thread_id,
            "funai",
            false,
            "json title",
        );
        fs::write(
            codex.join("session_index.jsonl"),
            format!("{{\"id\":\"{thread_id}\",\"thread_name\":\"renamed title\"}}\n"),
        )
        .unwrap();
        let state_db = codex.join("state_5.sqlite");
        init_state_db(&state_db);
        insert_state_row_with_title(&state_db, thread_id, "funai", 0, "sqlite title");
        let discovered_path = session_files(&codex).unwrap()[0].path.clone();
        set_state_paths(&state_db, thread_id, &discovered_path, "D:\\work");
        let catalog_db = codex.join("sqlite/codex-dev.db");
        init_catalog_db(&catalog_db);

        let result = preview_codex_catalog_repair(CatalogRepairRequest {
            codex_home: codex.display().to_string(),
            thread_ids: vec![thread_id.to_string()],
        })
        .unwrap();

        assert!(result.dry_run);
        assert!(result.backup_dir.is_none());
        assert_eq!(result.changed_threads, vec![thread_id.to_string()]);
        assert_eq!(result.planned_changes.len(), 1);
        assert_eq!(result.planned_changes[0].action, "insert_catalog_entry");
        assert_eq!(result.planned_changes[0].display_title, "renamed title");
        assert_eq!(result.planned_changes[0].cwd, "D:\\work");
        assert_eq!(
            result.planned_changes[0].model_provider.as_deref(),
            Some("funai")
        );
        let connection = Connection::open(&catalog_db).unwrap();
        let count: i64 = connection
            .query_row("select count(*) from local_thread_catalog", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn scan_selects_active_thread_when_index_or_state_visibility_is_missing() {
        let temp = tempfile::tempdir().unwrap();
        let codex = temp.path().join(".codex");
        let thread_id = "019f32e8-178a-7b01-9a43-61e5a75d73ae";
        write_jsonl(
            &codex.join("sessions/2026/07/05/rollout-a-019f32e8-178a-7b01-9a43-61e5a75d73ae.jsonl"),
            thread_id,
            "funai",
            false,
            "json title",
        );
        fs::write(codex.join("session_index.jsonl"), "").unwrap();
        init_state_db(&codex.join("state_5.sqlite"));
        let catalog_db = codex.join("sqlite/codex-dev.db");
        init_catalog_db(&catalog_db);
        insert_catalog_row(&catalog_db, thread_id, "json title");

        let response = scan_codex_catalog_repair(&codex).unwrap();

        assert_eq!(response.summary.missing_catalog_entries, 0);
        assert_eq!(response.summary.missing_session_index_entries, 1);
        assert_eq!(response.summary.state_metadata_issues, 1);
        assert_eq!(
            response.rows[0].repair_codes,
            vec!["missing_session_index", "missing_state_entry"]
        );
        assert!(response.rows[0].selected_by_default);
    }

    #[test]
    fn scan_selects_active_thread_when_state_paths_are_extended_windows_paths() {
        let temp = tempfile::tempdir().unwrap();
        let codex = temp.path().join(".codex");
        let thread_id = "019f32e8-178a-7b01-9a43-61e5a75d73ae";
        let jsonl_path =
            codex.join("sessions/2026/07/05/rollout-a-019f32e8-178a-7b01-9a43-61e5a75d73ae.jsonl");
        write_jsonl(&jsonl_path, thread_id, "funai", false, "json title");
        fs::write(
            codex.join("session_index.jsonl"),
            format!("{{\"id\":\"{thread_id}\",\"thread_name\":\"renamed title\"}}\n"),
        )
        .unwrap();
        let state_db = codex.join("state_5.sqlite");
        init_state_db(&state_db);
        insert_state_row_with_title(&state_db, thread_id, "funai", 0, "renamed title");
        poison_state_paths(&state_db, thread_id, &jsonl_path);
        let catalog_db = codex.join("sqlite/codex-dev.db");
        init_catalog_db(&catalog_db);
        insert_catalog_row(&catalog_db, thread_id, "renamed title");

        let response = scan_codex_catalog_repair(&codex).unwrap();

        assert_eq!(
            response.rows[0].repair_codes,
            vec!["state_cwd_mismatch", "state_rollout_path_mismatch"]
        );
        assert_eq!(response.summary.state_metadata_issues, 1);
        assert!(response.rows[0].selected_by_default);
    }

    #[test]
    fn scan_selects_active_thread_with_rollout_internal_passthrough_metadata() {
        let temp = tempfile::tempdir().unwrap();
        let codex = temp.path().join(".codex");
        let thread_id = "019f32e8-178a-7b01-9a43-61e5a75d73ae";
        let jsonl_path =
            codex.join("sessions/2026/07/05/rollout-a-019f32e8-178a-7b01-9a43-61e5a75d73ae.jsonl");
        write_jsonl_with_internal_passthrough(&jsonl_path, thread_id);
        fs::write(
            codex.join("session_index.jsonl"),
            format!("{{\"id\":\"{thread_id}\",\"thread_name\":\"renamed title\"}}\n"),
        )
        .unwrap();
        let discovered_path = session_files(&codex).unwrap()[0].path.clone();
        for state_db in [
            codex.join("state_5.sqlite"),
            codex.join("sqlite/state_5.sqlite"),
        ] {
            init_state_db(&state_db);
            insert_state_row_with_title(&state_db, thread_id, "funai", 0, "renamed title");
            set_state_paths(&state_db, thread_id, &discovered_path, "D:\\work");
        }
        let catalog_db = codex.join("sqlite/codex-dev.db");
        init_catalog_db(&catalog_db);
        insert_catalog_row(&catalog_db, thread_id, "renamed title");

        let response = scan_codex_catalog_repair(&codex).unwrap();

        assert_eq!(
            response.rows[0].repair_codes,
            vec!["rollout_internal_metadata_passthrough"]
        );
        assert!(response.rows[0].selected_by_default);
    }

    #[test]
    fn apply_refuses_when_codex_process_is_running() {
        let temp = tempfile::tempdir().unwrap();
        let codex = temp.path().join(".codex");
        let thread_id = "019f32e8-178a-7b01-9a43-61e5a75d73ae";
        write_jsonl(
            &codex.join("sessions/2026/07/05/rollout-a-019f32e8-178a-7b01-9a43-61e5a75d73ae.jsonl"),
            thread_id,
            "funai",
            false,
            "json title",
        );
        init_catalog_db(&codex.join("sqlite/codex-dev.db"));

        let error = apply_codex_catalog_repair_with_process_checker(
            CatalogRepairRequest {
                codex_home: codex.display().to_string(),
                thread_ids: vec![thread_id.to_string()],
            },
            || true,
        )
        .unwrap_err();

        assert_eq!(error.code, "codex_process_running");
    }

    #[test]
    fn apply_inserts_catalog_row_after_backup_and_keeps_jsonl_unchanged() {
        let temp = tempfile::tempdir().unwrap();
        let codex = temp.path().join(".codex");
        let thread_id = "019f32e8-178a-7b01-9a43-61e5a75d73ae";
        let jsonl_path =
            codex.join("sessions/2026/07/05/rollout-a-019f32e8-178a-7b01-9a43-61e5a75d73ae.jsonl");
        write_jsonl(&jsonl_path, thread_id, "funai", false, "json title");
        fs::write(
            codex.join("session_index.jsonl"),
            format!("{{\"id\":\"{thread_id}\",\"thread_name\":\"renamed title\"}}\n"),
        )
        .unwrap();
        let catalog_db = codex.join("sqlite/codex-dev.db");
        init_catalog_db(&catalog_db);
        let original_jsonl = fs::read(&jsonl_path).unwrap();

        let result = apply_codex_catalog_repair_with_process_checker(
            CatalogRepairRequest {
                codex_home: codex.display().to_string(),
                thread_ids: vec![thread_id.to_string()],
            },
            || false,
        )
        .unwrap();

        assert!(!result.dry_run);
        assert_eq!(result.changed_threads, vec![thread_id.to_string()]);
        let backup_dir = result.backup_dir.as_deref().unwrap();
        assert!(std::path::Path::new(backup_dir).exists());
        let connection = Connection::open(&catalog_db).unwrap();
        let row: (String, String, String, i32) = connection
            .query_row(
                "select display_title, cwd, model_provider, missing_candidate from local_thread_catalog where thread_id=?1",
                [thread_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .unwrap();
        assert_eq!(
            row,
            (
                "renamed title".to_string(),
                "D:\\work".to_string(),
                "funai".to_string(),
                0
            )
        );
        assert_eq!(fs::read(&jsonl_path).unwrap(), original_jsonl);
    }

    #[test]
    fn apply_repairs_existing_catalog_row_index_and_state_after_backup() {
        let temp = tempfile::tempdir().unwrap();
        let codex = temp.path().join(".codex");
        let thread_id = "019f32e8-178a-7b01-9a43-61e5a75d73ae";
        let jsonl_path =
            codex.join("sessions/2026/07/05/rollout-a-019f32e8-178a-7b01-9a43-61e5a75d73ae.jsonl");
        write_jsonl(&jsonl_path, thread_id, "funai", false, "json title");
        fs::write(codex.join("session_index.jsonl"), "").unwrap();
        init_state_db(&codex.join("state_5.sqlite"));
        init_state_db(&codex.join("sqlite/state_5.sqlite"));
        let catalog_db = codex.join("sqlite/codex-dev.db");
        init_catalog_db(&catalog_db);
        insert_catalog_row(&catalog_db, thread_id, "json title");
        let original_jsonl = fs::read(&jsonl_path).unwrap();

        let result = apply_codex_catalog_repair_with_process_checker(
            CatalogRepairRequest {
                codex_home: codex.display().to_string(),
                thread_ids: vec![thread_id.to_string()],
            },
            || false,
        )
        .unwrap();

        assert!(!result.dry_run);
        assert_eq!(result.changed_threads, vec![thread_id.to_string()]);
        assert!(result.backup_dir.is_some());
        assert!(fs::read_to_string(codex.join("session_index.jsonl"))
            .unwrap()
            .contains(thread_id));
        for state_db in [
            codex.join("state_5.sqlite"),
            codex.join("sqlite/state_5.sqlite"),
        ] {
            let connection = Connection::open(state_db).unwrap();
            let row: (String, String, i32) = connection
                .query_row(
                    "select title, model_provider, archived from threads where id=?1",
                    [thread_id],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                )
                .unwrap();
            assert_eq!(row, ("json title".to_string(), "funai".to_string(), 0));
        }
        assert_eq!(fs::read(&jsonl_path).unwrap(), original_jsonl);
    }

    #[test]
    fn apply_repairs_existing_state_path_and_cwd_after_backup() {
        let temp = tempfile::tempdir().unwrap();
        let codex = temp.path().join(".codex");
        let thread_id = "019f32e8-178a-7b01-9a43-61e5a75d73ae";
        let jsonl_path =
            codex.join("sessions/2026/07/05/rollout-a-019f32e8-178a-7b01-9a43-61e5a75d73ae.jsonl");
        write_jsonl(&jsonl_path, thread_id, "funai", false, "json title");
        fs::write(
            codex.join("session_index.jsonl"),
            format!("{{\"id\":\"{thread_id}\",\"thread_name\":\"renamed title\"}}\n"),
        )
        .unwrap();
        for state_db in [
            codex.join("state_5.sqlite"),
            codex.join("sqlite/state_5.sqlite"),
        ] {
            init_state_db(&state_db);
            insert_state_row_with_title(&state_db, thread_id, "funai", 0, "renamed title");
            poison_state_paths(&state_db, thread_id, &jsonl_path);
        }
        let catalog_db = codex.join("sqlite/codex-dev.db");
        init_catalog_db(&catalog_db);
        insert_catalog_row(&catalog_db, thread_id, "renamed title");
        let original_jsonl = fs::read(&jsonl_path).unwrap();

        let result = apply_codex_catalog_repair_with_process_checker(
            CatalogRepairRequest {
                codex_home: codex.display().to_string(),
                thread_ids: vec![thread_id.to_string()],
            },
            || false,
        )
        .unwrap();

        assert!(!result.dry_run);
        assert_eq!(result.changed_threads, vec![thread_id.to_string()]);
        assert_eq!(result.planned_changes.len(), 1);
        assert_eq!(result.planned_changes[0].action, "upsert_state_entry");
        for state_db in [
            codex.join("state_5.sqlite"),
            codex.join("sqlite/state_5.sqlite"),
        ] {
            let connection = Connection::open(state_db).unwrap();
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
                    visible_path_string(&session_files(&codex).unwrap()[0].path),
                    "D:\\work".to_string(),
                    "renamed title".to_string(),
                )
            );
        }
        assert_eq!(fs::read(&jsonl_path).unwrap(), original_jsonl);
    }

    #[test]
    fn apply_strips_rollout_internal_passthrough_metadata_after_backup() {
        let temp = tempfile::tempdir().unwrap();
        let codex = temp.path().join(".codex");
        let thread_id = "019f32e8-178a-7b01-9a43-61e5a75d73ae";
        let jsonl_path =
            codex.join("sessions/2026/07/05/rollout-a-019f32e8-178a-7b01-9a43-61e5a75d73ae.jsonl");
        write_jsonl_with_internal_passthrough(&jsonl_path, thread_id);
        fs::write(
            codex.join("session_index.jsonl"),
            format!("{{\"id\":\"{thread_id}\",\"thread_name\":\"renamed title\"}}\n"),
        )
        .unwrap();
        let discovered_path = session_files(&codex).unwrap()[0].path.clone();
        for state_db in [
            codex.join("state_5.sqlite"),
            codex.join("sqlite/state_5.sqlite"),
        ] {
            init_state_db(&state_db);
            insert_state_row_with_title(&state_db, thread_id, "funai", 0, "renamed title");
            set_state_paths(&state_db, thread_id, &discovered_path, "D:\\work");
        }
        let catalog_db = codex.join("sqlite/codex-dev.db");
        init_catalog_db(&catalog_db);
        insert_catalog_row(&catalog_db, thread_id, "renamed title");
        let original_jsonl = fs::read_to_string(&jsonl_path).unwrap();

        let result = apply_codex_catalog_repair_with_process_checker(
            CatalogRepairRequest {
                codex_home: codex.display().to_string(),
                thread_ids: vec![thread_id.to_string()],
            },
            || false,
        )
        .unwrap();

        assert_eq!(result.planned_changes.len(), 1);
        assert_eq!(
            result.planned_changes[0].action,
            "strip_rollout_internal_metadata_passthrough"
        );
        let repaired_jsonl = fs::read_to_string(&jsonl_path).unwrap();
        assert!(!repaired_jsonl.contains("internal_chat_message_metadata_passthrough"));
        assert!(repaired_jsonl.contains("\"text\":\"hello\""));
        let backup_dir = std::path::Path::new(result.backup_dir.as_deref().unwrap());
        let backup_jsonl = backup_dir
            .join("sessions/2026/07/05/rollout-a-019f32e8-178a-7b01-9a43-61e5a75d73ae.jsonl");
        assert_eq!(fs::read_to_string(backup_jsonl).unwrap(), original_jsonl);
    }
}

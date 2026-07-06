use crate::codex::metadata::{metadata_from_bytes, SessionMetadata};
use crate::codex::scan::session_files;
use crate::codex::sqlite::{state_dbs, state_entry};
use crate::codex::{CommandError, Result, ThreadLifecycle};
use rusqlite::{Connection, OpenFlags};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

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

#[derive(Debug, Clone)]
struct CatalogEntry {
    display_title: Option<String>,
    cwd: Option<String>,
}

pub fn scan_codex_catalog_repair(codex_home: &Path) -> Result<CatalogRepairScanResponse> {
    validate_codex_home(codex_home)?;

    let index_titles = index_titles(codex_home)?;
    let catalog_db = codex_home.join("sqlite/codex-dev.db");
    let catalog_entries = read_catalog_entries(&catalog_db)?;
    let catalog_db_path = catalog_db
        .exists()
        .then(|| catalog_db.display().to_string());
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
        let state_title = first_state_title(codex_home, &metadata.thread_id);
        let display_title = display_title(&metadata, &index_titles, state_title.as_deref());
        let mut repair_codes = Vec::new();
        if !catalog_db.exists() {
            repair_codes.push("catalog_db_missing".to_string());
        } else if let Some(entry) = catalog_entries.get(&metadata.thread_id) {
            if entry.cwd.as_deref().unwrap_or_default() != metadata.cwd {
                repair_codes.push("catalog_cwd_mismatch".to_string());
            }
            if entry.display_title.as_deref().unwrap_or_default().trim().is_empty()
                && !display_title.trim().is_empty()
            {
                repair_codes.push("catalog_title_stale".to_string());
            }
        } else {
            repair_codes.push("missing_catalog_entry".to_string());
        }
        repair_codes.sort();
        repair_codes.dedup();
        let selected_by_default = file.lifecycle == ThreadLifecycle::Active
            && repair_codes
                .iter()
                .any(|code| code == "missing_catalog_entry");
        rows.push(CatalogRepairRow {
            thread_id: metadata.thread_id.clone(),
            display_title,
            lifecycle: file.lifecycle,
            project_name: project_name_from_cwd(&metadata.cwd),
            project_path: non_empty_project_path(&metadata.cwd),
            path: metadata.path.display().to_string(),
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
    cwd.trim()
        .trim_end_matches(['/', '\\'])
        .split(['/', '\\'])
        .filter(|part| !part.is_empty())
        .next_back()
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(str::to_string)
}

fn non_empty_project_path(cwd: &str) -> Option<String> {
    let trimmed = cwd.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

fn lifecycle_rank(lifecycle: &ThreadLifecycle) -> u8 {
    match lifecycle {
        ThreadLifecycle::Active => 0,
        ThreadLifecycle::Archived => 1,
    }
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
        init_catalog_db(&codex.join("sqlite/codex-dev.db"));

        let response = scan_codex_catalog_repair(&codex).unwrap();

        assert_eq!(response.summary.total_threads, 1);
        assert_eq!(response.summary.missing_catalog_entries, 1);
        assert_eq!(response.rows[0].thread_id, thread_id);
        assert_eq!(response.rows[0].display_title, "renamed title");
        assert_eq!(response.rows[0].repair_codes, vec!["missing_catalog_entry"]);
        assert!(response.rows[0].selected_by_default);
    }
}

use crate::codex::metadata::{metadata_from_bytes, SessionMetadata, BOM};
use crate::codex::paths::{normalize_windows_extended_path, visible_path_string};
use crate::codex::sqlite::{state_dbs, state_entry};
use crate::codex::{
    CommandError, DashboardModel, ProviderOption, ProviderOptionKind, ProviderOptions, Result,
    ScanResponse, ThreadLifecycle, ThreadRow,
};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionFile {
    pub path: PathBuf,
    pub lifecycle: ThreadLifecycle,
}

pub fn scan_codex_home(codex_home: &Path) -> Result<ScanResponse> {
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

    let config_provider = config_provider(codex_home)?;
    let index_entries = index_entries(codex_home)?;
    let index_ids: BTreeSet<String> = index_entries.keys().cloned().collect();
    let dbs = state_dbs(codex_home);
    let mut rows = Vec::new();
    let mut source_providers = BTreeSet::new();

    let files = session_files(codex_home)?;
    if files.is_empty() {
        return Err(CommandError::new(
            "no_sessions_found",
            format!("No Codex sessions found in {}", sessions_dir.display()),
        ));
    }

    for file in files {
        let raw = fs::read(&file.path)
            .map_err(|error| CommandError::io("read session", file.path.display(), error))?;
        let metadata = metadata_from_bytes(&raw, &file.path)?;
        if let Some(provider) = &metadata.provider {
            source_providers.insert(provider.clone());
        }
        let mut issue_codes = Vec::new();
        if raw.starts_with(BOM) {
            issue_codes.push("bom_present".to_string());
        }
        if let (Some(config), Some(file)) = (&config_provider, &metadata.provider) {
            if config != file {
                issue_codes.push("provider_mismatch".to_string());
            }
        }
        if !index_ids.contains(&metadata.thread_id) {
            issue_codes.push("missing_index".to_string());
        }
        let mut state_title = None;
        for db in &dbs {
            let entry = state_entry(db, &metadata.thread_id);
            if !entry.exists {
                issue_codes.push("missing_state_entry".to_string());
            } else {
                if state_title.is_none() {
                    state_title = entry.title.clone();
                }
                if let (Some(state_provider), Some(file_provider)) =
                    (&entry.provider, &metadata.provider)
                {
                    if state_provider != file_provider {
                        issue_codes.push("state_provider_mismatch".to_string());
                    }
                }
                if entry.archived.unwrap_or(0) != 0 {
                    issue_codes.push("archived_state".to_string());
                }
            }
        }
        issue_codes.sort();
        issue_codes.dedup();
        rows.push(thread_row(
            &metadata,
            display_title(&metadata, &index_entries, state_title.as_deref()),
            config_provider.clone(),
            file.lifecycle,
            issue_codes,
        ));
    }

    rows.sort_by(|left, right| {
        lifecycle_rank(&left.lifecycle)
            .cmp(&lifecycle_rank(&right.lifecycle))
            .then_with(|| right.updated_at_ms.cmp(&left.updated_at_ms))
            .then_with(|| right.severity.cmp(&left.severity))
            .then_with(|| left.thread_id.cmp(&right.thread_id))
    });
    let mut issue_counts = BTreeMap::new();
    for row in &rows {
        for code in &row.issue_codes {
            *issue_counts.entry(code.clone()).or_insert(0) += 1;
        }
    }
    let dashboard = DashboardModel {
        codex_home: codex_home.display().to_string(),
        total_threads: rows.len(),
        problem_threads: rows
            .iter()
            .filter(|row| !row.issue_codes.is_empty())
            .count(),
        issue_counts,
        rows,
    };
    let provider_options = provider_options(config_provider.clone(), source_providers);
    Ok(ScanResponse {
        dashboard,
        provider_options,
        config_provider,
    })
}

pub fn session_files(codex_home: &Path) -> Result<Vec<SessionFile>> {
    let mut files = Vec::new();
    let sessions_dir = codex_home.join("sessions");
    if sessions_dir.exists() {
        visit_session_dir(&sessions_dir, ThreadLifecycle::Active, &mut files)?;
    }
    let archived_dir = codex_home.join("archived_sessions");
    if archived_dir.exists() {
        visit_session_dir(&archived_dir, ThreadLifecycle::Archived, &mut files)?;
    }
    files.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(files)
}

fn visit_session_dir(
    dir: &Path,
    lifecycle: ThreadLifecycle,
    files: &mut Vec<SessionFile>,
) -> Result<()> {
    for entry in fs::read_dir(dir)
        .map_err(|error| CommandError::io("read sessions directory", dir.display(), error))?
    {
        let entry =
            entry.map_err(|error| CommandError::io("read session entry", dir.display(), error))?;
        let path = entry.path();
        if path.is_dir() {
            visit_session_dir(&path, lifecycle.clone(), files)?;
        } else if path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with("rollout-") && name.ends_with(".jsonl"))
        {
            files.push(SessionFile {
                path,
                lifecycle: lifecycle.clone(),
            });
        }
    }
    Ok(())
}

pub fn config_provider(codex_home: &Path) -> Result<Option<String>> {
    let path = codex_home.join("config.toml");
    if !path.exists() {
        return Ok(None);
    }
    let text = fs::read_to_string(&path)
        .map_err(|error| CommandError::io("read config", path.display(), error))?;
    for line in text.lines() {
        let line = line.split('#').next().unwrap_or_default().trim();
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        if key.trim() != "model_provider" {
            continue;
        }
        let value = value.trim();
        if let Some(value) = quoted_toml_string(value) {
            return Ok(Some(value));
        }
    }
    Ok(None)
}

fn quoted_toml_string(value: &str) -> Option<String> {
    let mut chars = value.chars();
    if chars.next()? != '"' {
        return None;
    }
    let mut parsed = String::new();
    let mut escaped = false;
    for character in chars {
        if escaped {
            parsed.push(character);
            escaped = false;
            continue;
        }
        if character == '\\' {
            escaped = true;
            continue;
        }
        if character == '"' {
            return (!parsed.is_empty()).then_some(parsed);
        }
        parsed.push(character);
    }
    None
}

pub fn index_ids(codex_home: &Path) -> Result<BTreeSet<String>> {
    Ok(index_entries(codex_home)?.keys().cloned().collect())
}

fn index_entries(codex_home: &Path) -> Result<BTreeMap<String, String>> {
    let path = codex_home.join("session_index.jsonl");
    if !path.exists() {
        return Ok(BTreeMap::new());
    }
    let text = fs::read_to_string(&path)
        .map_err(|error| CommandError::io("read session index", path.display(), error))?;
    let mut entries = BTreeMap::new();
    for line in text.lines().filter(|line| !line.trim().is_empty()) {
        let Ok(value) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        if let Some(id) = value.get("id").and_then(Value::as_str) {
            let title = value
                .get("thread_name")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .unwrap_or_default()
                .to_string();
            entries.insert(id.to_string(), title);
        }
    }
    Ok(entries)
}

fn display_title(
    metadata: &SessionMetadata,
    index_entries: &BTreeMap<String, String>,
    state_title: Option<&str>,
) -> String {
    index_entries
        .get(&metadata.thread_id)
        .filter(|title| !title.is_empty())
        .cloned()
        .or_else(|| {
            state_title
                .map(str::trim)
                .filter(|title| !title.is_empty())
                .map(str::to_string)
        })
        .unwrap_or_else(|| metadata.title.clone())
}

fn thread_row(
    metadata: &SessionMetadata,
    display_name: String,
    config_provider: Option<String>,
    lifecycle: ThreadLifecycle,
    issue_codes: Vec<String>,
) -> ThreadRow {
    let suggested_action_code = suggested_action_code(&issue_codes);
    let suggested_action_values = suggested_action_values(
        &suggested_action_code,
        metadata.provider.as_deref(),
        config_provider.as_deref(),
    );
    ThreadRow {
        thread_id: metadata.thread_id.clone(),
        short_id: metadata.thread_id.chars().take(8).collect(),
        display_name,
        project_name: project_name_from_cwd(&metadata.cwd),
        project_path: non_empty_project_path(&metadata.cwd),
        path: visible_path_string(&metadata.path),
        file_provider: metadata.provider.clone(),
        config_provider,
        lifecycle,
        severity: issue_codes
            .iter()
            .map(|code| severity(code))
            .max()
            .unwrap_or(0),
        can_migrate: metadata.provider.is_some(),
        suggested_action_code,
        suggested_action_values,
        updated_at_ms: metadata.updated_at_ms,
        issue_codes,
    }
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

fn severity(code: &str) -> i32 {
    match code {
        "bom_present" => 100,
        "missing_state_entry" => 90,
        "state_provider_mismatch" => 80,
        "provider_mismatch" => 70,
        "missing_index" => 60,
        "archived_state" => 50,
        _ => 10,
    }
}

fn suggested_action_code(codes: &[String]) -> String {
    if codes.iter().any(|code| {
        matches!(
            code.as_str(),
            "bom_present" | "provider_mismatch" | "state_provider_mismatch"
        )
    }) {
        "migrate_provider".to_string()
    } else if codes
        .iter()
        .any(|code| matches!(code.as_str(), "missing_state_entry" | "missing_index"))
    {
        "rebuild_visibility_metadata".to_string()
    } else if codes.iter().any(|code| code == "archived_state") {
        "unarchive_sqlite_thread".to_string()
    } else {
        "no_action_needed".to_string()
    }
}

fn suggested_action_values(
    action: &str,
    file_provider: Option<&str>,
    config_provider: Option<&str>,
) -> BTreeMap<String, String> {
    let mut values = BTreeMap::new();
    if action == "migrate_provider" {
        values.insert(
            "source".to_string(),
            file_provider.unwrap_or("<source-provider>").to_string(),
        );
        values.insert(
            "target".to_string(),
            config_provider.unwrap_or("<target-provider>").to_string(),
        );
    }
    values
}

fn provider_options(
    config_provider: Option<String>,
    source_providers: BTreeSet<String>,
) -> ProviderOptions {
    let mut target_providers = Vec::new();
    if let Some(config) = &config_provider {
        target_providers.push(ProviderOption {
            value: config.clone(),
            label: format!("{config}（当前配置，推荐）"),
            kind: ProviderOptionKind::Config,
            recommended: true,
        });
    }
    for provider in &source_providers {
        if Some(provider) == config_provider.as_ref() {
            continue;
        }
        target_providers.push(ProviderOption {
            value: provider.clone(),
            label: provider.clone(),
            kind: ProviderOptionKind::Discovered,
            recommended: false,
        });
    }
    ProviderOptions {
        current_config_provider: config_provider,
        source_providers: source_providers.into_iter().collect(),
        target_providers,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codex::test_support::{
        init_state_db, insert_state_row, insert_state_row_with_title, write_jsonl,
    };
    use crate::codex::ThreadLifecycle;

    #[test]
    fn scan_builds_dashboard_and_provider_options() {
        let temp = tempfile::tempdir().unwrap();
        let codex = temp.path().join(".codex");
        let wanted = "019eca3b-941d-7340-9b14-328c635a6523";
        write_jsonl(
            &codex.join("sessions/2026/06/15/rollout-a-019eca3b-941d-7340-9b14-328c635a6523.jsonl"),
            wanted,
            "funai",
            true,
            "你好，迁移 provider",
        );
        write_jsonl(
            &codex.join("sessions/2026/06/15/rollout-b-019ec94d-720d-7a12-a379-28c8042bc6b4.jsonl"),
            "019ec94d-720d-7a12-a379-28c8042bc6b4",
            "gmn",
            false,
            "另一个会话",
        );
        fs::write(
            codex.join("config.toml"),
            "model_provider = \"yihubangg\"\n",
        )
        .unwrap();
        fs::write(codex.join("session_index.jsonl"), "").unwrap();
        init_state_db(&codex.join("state_5.sqlite"));

        let response = scan_codex_home(&codex).unwrap();

        assert_eq!(response.config_provider.as_deref(), Some("yihubangg"));
        assert_eq!(response.dashboard.total_threads, 2);
        assert_eq!(response.dashboard.problem_threads, 2);
        assert!(response
            .provider_options
            .source_providers
            .contains(&"funai".to_string()));
        assert!(response
            .provider_options
            .source_providers
            .contains(&"gmn".to_string()));
        assert_eq!(
            response.provider_options.target_providers[0].value,
            "yihubangg"
        );
        assert_eq!(
            response.provider_options.target_providers[0].label,
            "yihubangg（当前配置，推荐）"
        );
        assert!(response.provider_options.target_providers[0].recommended);
        let first = response
            .dashboard
            .rows
            .iter()
            .find(|row| row.thread_id == wanted)
            .unwrap();
        assert!(first.issue_codes.contains(&"bom_present".to_string()));
        assert!(first.issue_codes.contains(&"provider_mismatch".to_string()));
        assert!(first.issue_codes.contains(&"missing_index".to_string()));
        assert!(first
            .issue_codes
            .contains(&"missing_state_entry".to_string()));
        assert_eq!(first.display_name, "你好，迁移 provider");
    }

    #[test]
    fn scan_reports_when_no_sessions_are_found() {
        let temp = tempfile::tempdir().unwrap();
        let codex = temp.path().join(".codex");
        fs::create_dir_all(codex.join("sessions")).unwrap();

        let error = scan_codex_home(&codex).unwrap_err();

        assert_eq!(error.code, "no_sessions_found");
    }

    #[test]
    fn scan_includes_archived_sessions_and_sorts_active_threads_first() {
        let temp = tempfile::tempdir().unwrap();
        let codex = temp.path().join(".codex");
        let active_id = "019ec94d-720d-7a12-a379-28c8042bc6b4";
        let archived_id = "019eca3b-941d-7340-9b14-328c635a6523";
        let state_db = codex.join("state_5.sqlite");
        write_jsonl(
            &codex.join("archived_sessions/rollout-a-019eca3b-941d-7340-9b14-328c635a6523.jsonl"),
            archived_id,
            "funai",
            false,
            "已归档会话",
        );
        write_jsonl(
            &codex.join("sessions/2026/06/15/rollout-b-019ec94d-720d-7a12-a379-28c8042bc6b4.jsonl"),
            active_id,
            "funai",
            false,
            "活跃会话",
        );
        fs::write(codex.join("config.toml"), "model_provider = \"funai\"\n").unwrap();
        fs::write(
            codex.join("session_index.jsonl"),
            format!("{{\"id\":\"{active_id}\"}}\n{{\"id\":\"{archived_id}\"}}\n"),
        )
        .unwrap();
        init_state_db(&state_db);
        insert_state_row(&state_db, active_id, "funai", 0);
        insert_state_row(&state_db, archived_id, "funai", 1);

        let response = scan_codex_home(&codex).unwrap();

        assert_eq!(response.dashboard.rows[0].thread_id, active_id);
        assert_eq!(
            response.dashboard.rows[0].lifecycle,
            ThreadLifecycle::Active
        );
        assert_eq!(response.dashboard.rows[1].thread_id, archived_id);
        assert_eq!(
            response.dashboard.rows[1].lifecycle,
            ThreadLifecycle::Archived
        );
        assert!(response.dashboard.rows[1]
            .issue_codes
            .contains(&"archived_state".to_string()));
    }

    #[test]
    fn scan_prefers_renamed_session_title_from_index() {
        let temp = tempfile::tempdir().unwrap();
        let codex = temp.path().join(".codex");
        let thread_id = "019eee11-9343-7f30-971e-b01e55a058c8";
        let state_db = codex.join("state_5.sqlite");
        write_jsonl(
            &codex.join("sessions/2026/06/22/rollout-a-019eee11-9343-7f30-971e-b01e55a058c8.jsonl"),
            thread_id,
            "funai",
            false,
            "old generated title",
        );
        fs::write(codex.join("config.toml"), "model_provider = \"funai\"\n").unwrap();
        fs::write(
            codex.join("session_index.jsonl"),
            format!("{{\"id\":\"{thread_id}\",\"thread_name\":\"renamed title\"}}\n"),
        )
        .unwrap();
        init_state_db(&state_db);
        insert_state_row(&state_db, thread_id, "funai", 0);

        let response = scan_codex_home(&codex).unwrap();

        assert_eq!(response.dashboard.rows[0].display_name, "renamed title");
    }

    #[test]
    fn scan_uses_sqlite_title_when_index_has_no_thread_name() {
        let temp = tempfile::tempdir().unwrap();
        let codex = temp.path().join(".codex");
        let thread_id = "019eee31-9343-7f30-971e-b01e55a058c8";
        let state_db = codex.join("state_5.sqlite");
        write_jsonl(
            &codex.join("sessions/2026/06/22/rollout-a-019eee31-9343-7f30-971e-b01e55a058c8.jsonl"),
            thread_id,
            "funai",
            false,
            "old generated title",
        );
        fs::write(codex.join("config.toml"), "model_provider = \"funai\"\n").unwrap();
        fs::write(
            codex.join("session_index.jsonl"),
            format!("{{\"id\":\"{thread_id}\"}}\n"),
        )
        .unwrap();
        init_state_db(&state_db);
        insert_state_row_with_title(&state_db, thread_id, "funai", 0, "sqlite renamed title");

        let response = scan_codex_home(&codex).unwrap();

        assert_eq!(
            response.dashboard.rows[0].display_name,
            "sqlite renamed title"
        );
    }

    #[test]
    fn scan_reports_project_name_from_session_cwd() {
        let temp = tempfile::tempdir().unwrap();
        let codex = temp.path().join(".codex");
        let thread_id = "019eee41-9343-7f30-971e-b01e55a058c8";
        let state_db = codex.join("state_5.sqlite");
        write_jsonl(
            &codex.join("sessions/2026/06/22/rollout-a-019eee41-9343-7f30-971e-b01e55a058c8.jsonl"),
            thread_id,
            "funai",
            false,
            "project scoped title",
        );
        fs::write(codex.join("config.toml"), "model_provider = \"funai\"\n").unwrap();
        fs::write(
            codex.join("session_index.jsonl"),
            format!("{{\"id\":\"{thread_id}\"}}\n"),
        )
        .unwrap();
        init_state_db(&state_db);
        insert_state_row(&state_db, thread_id, "funai", 0);

        let response = scan_codex_home(&codex).unwrap();
        let row = serde_json::to_value(&response.dashboard.rows[0]).unwrap();

        assert_eq!(row["projectName"], "work");
        assert_eq!(row["projectPath"], "D:\\work");
    }

    #[test]
    fn config_provider_reads_toml_value_without_inline_comment() {
        let temp = tempfile::tempdir().unwrap();
        let codex = temp.path().join(".codex");
        fs::create_dir_all(&codex).unwrap();
        fs::write(
            codex.join("config.toml"),
            "\
# model_provider = \"commented-out\"
model_provider_extra = \"wrong\"
model_provider = \"yihubangg\" # current desktop provider
",
        )
        .unwrap();

        let provider = config_provider(&codex).unwrap();

        assert_eq!(provider.as_deref(), Some("yihubangg"));
    }
}

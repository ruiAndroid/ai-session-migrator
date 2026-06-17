# Provider Migration Desktop Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the desktop prototype with a real Codex Desktop provider migration workflow backed by Rust/Tauri commands.

**Architecture:** `ai-session-doctor` remains the behavior reference, but the desktop app owns the runtime implementation. Rust scans and mutates the local Codex store, Tauri exposes typed commands, and React renders a Chinese desktop workflow with source and target provider dropdowns, dry-run preview, selection, confirmation, and result reporting.

**Tech Stack:** Tauri 2, Rust 2021, `serde`, `serde_json`, `regex`, `rusqlite`, `chrono`, React 18, TypeScript, Vite, Vitest, Testing Library.

---

## File Structure

Create or modify these files:

- `README.md`: keep existing product/development copy; include the real desktop workflow after implementation.
- `.gitignore`: already ignores `.superpowers/`, `node_modules/`, `dist/`, and Rust build output.
- `package.json`, `package-lock.json`, `apps/desktop/package.json`, `apps/desktop/tsconfig.json`, `apps/desktop/vitest.config.ts`, `apps/desktop/index.html`, `apps/desktop/src/main.tsx`, `apps/desktop/src/test/setup.ts`: keep scaffold; commit as baseline before behavior work.
- `apps/desktop/src-tauri/Cargo.toml`: add Rust dependencies for scanning, JSON, regex, sqlite, timestamp parsing, and tests.
- `apps/desktop/src-tauri/src/lib.rs`: expose the Rust library modules so `cargo test --lib` can run without launching Tauri.
- `apps/desktop/src-tauri/src/main.rs`: keep Tauri bootstrap small; register commands from the library.
- `apps/desktop/src-tauri/src/codex/mod.rs`: public Rust data contracts and command-facing functions.
- `apps/desktop/src-tauri/src/codex/error.rs`: structured command errors.
- `apps/desktop/src-tauri/src/codex/metadata.rs`: parse JSONL bytes and extract Codex session metadata.
- `apps/desktop/src-tauri/src/codex/scan.rs`: scan Codex home, build dashboard rows, and aggregate provider options.
- `apps/desktop/src-tauri/src/codex/sqlite.rs`: read and update `state_5.sqlite` thread rows.
- `apps/desktop/src-tauri/src/codex/backup.rs`: create backup directories and copy affected files.
- `apps/desktop/src-tauri/src/codex/migration.rs`: dry-run and apply provider migration.
- `apps/desktop/src-tauri/src/codex/test_support.rs`: Rust test fixtures for local `.codex` stores.
- `apps/desktop/src/domain/session.ts`: replace prototype rows with shared TypeScript command contracts.
- `apps/desktop/src/domain/migrationApi.ts`: thin adapter over Tauri `invoke`, injectable in tests.
- `apps/desktop/src/App.tsx`: real scan/preview/apply state machine and Chinese UI copy.
- `apps/desktop/src/App.test.tsx`: workflow tests using a fake migration API.
- `apps/desktop/src/styles.css`: update styling for selects, custom provider input, result panel, disabled states, and error messages.

## Task 0: Commit Existing Scaffold Baseline

**Files:**
- Add: `README.md`
- Add: `package.json`
- Add: `package-lock.json`
- Add: `docs/superpowers/plans/2026-06-15-desktop-mvp.md`
- Add: `apps/desktop/**`

- [ ] **Step 1: Confirm current untracked scaffold**

Run:

```powershell
git status --short
```

Expected: `README.md`, `package.json`, `package-lock.json`, `apps/`, and `docs/superpowers/plans/` are untracked. `.superpowers/`, `node_modules/`, `dist/`, and `target/` must not appear as normal untracked files.

- [ ] **Step 2: Run current frontend baseline**

Run:

```powershell
npm test
```

Expected: existing Vitest suite passes. The visible text is mojibake today, but this baseline proves the scaffold can execute before replacing the prototype.

- [ ] **Step 3: Stage only scaffold files**

Run:

```powershell
git add README.md package.json package-lock.json docs/superpowers/plans/2026-06-15-desktop-mvp.md apps/desktop
git status --short
```

Expected staged files include the scaffold. Ignored dependency/build folders remain ignored.

- [ ] **Step 4: Commit baseline**

Run:

```powershell
git commit -m "chore: commit desktop scaffold baseline"
```

Expected: commit succeeds and leaves the worktree ready for behavior changes.

## Task 1: Rust Contracts and Error Shape

**Files:**
- Modify: `apps/desktop/src-tauri/Cargo.toml`
- Create: `apps/desktop/src-tauri/src/lib.rs`
- Create: `apps/desktop/src-tauri/src/codex/mod.rs`
- Create: `apps/desktop/src-tauri/src/codex/error.rs`
- Modify: `apps/desktop/src-tauri/src/main.rs`

- [ ] **Step 1: Add Rust dependencies**

Modify `apps/desktop/src-tauri/Cargo.toml` so dependency sections contain:

```toml
[dependencies]
tauri = { version = "2", features = [] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
regex = "1"
rusqlite = { version = "0.32", features = ["bundled"] }
chrono = { version = "0.4", default-features = false, features = ["clock", "std"] }

[dev-dependencies]
tempfile = "3"
```

- [ ] **Step 2: Create library entry**

Create `apps/desktop/src-tauri/src/lib.rs`:

```rust
pub mod codex;
```

- [ ] **Step 3: Define command errors**

Create `apps/desktop/src-tauri/src/codex/error.rs`:

```rust
use serde::Serialize;
use std::fmt;
use std::io;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CommandError {
    pub code: String,
    pub message: String,
}

impl CommandError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }

    pub fn io(action: &str, path: impl fmt::Display, source: io::Error) -> Self {
        Self::new(
            "io_error",
            format!("{action}: {path} ({source})"),
        )
    }
}

impl fmt::Display for CommandError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for CommandError {}

pub type Result<T> = std::result::Result<T, CommandError>;
```

- [ ] **Step 4: Define shared Rust contracts**

Create `apps/desktop/src-tauri/src/codex/mod.rs`:

```rust
pub mod backup;
pub mod error;
pub mod metadata;
pub mod migration;
pub mod scan;
pub mod sqlite;

#[cfg(test)]
pub mod test_support;

pub use error::{CommandError, Result};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ThreadRow {
    pub thread_id: String,
    pub short_id: String,
    pub display_name: String,
    pub path: String,
    pub file_provider: Option<String>,
    pub config_provider: Option<String>,
    pub issue_codes: Vec<String>,
    pub severity: i32,
    pub can_migrate: bool,
    pub suggested_action_code: String,
    pub suggested_action_values: std::collections::BTreeMap<String, String>,
    pub updated_at_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DashboardModel {
    pub codex_home: String,
    pub total_threads: usize,
    pub problem_threads: usize,
    pub issue_counts: std::collections::BTreeMap<String, usize>,
    pub rows: Vec<ThreadRow>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProviderOption {
    pub value: String,
    pub label: String,
    pub kind: ProviderOptionKind,
    pub recommended: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ProviderOptionKind {
    Config,
    Discovered,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProviderOptions {
    pub current_config_provider: Option<String>,
    pub source_providers: Vec<String>,
    pub target_providers: Vec<ProviderOption>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ScanResponse {
    pub dashboard: DashboardModel,
    pub provider_options: ProviderOptions,
    pub config_provider: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MigrationRequest {
    pub codex_home: String,
    pub source_provider: Option<String>,
    pub target_provider: String,
    pub thread_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PlannedRepair {
    pub thread_id: String,
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MigrationResult {
    pub changed_threads: Vec<String>,
    pub planned_repairs: Vec<PlannedRepair>,
    pub backup_dir: Option<String>,
    pub dry_run: bool,
}

pub fn scan_codex_home(codex_home: String) -> Result<ScanResponse> {
    scan::scan_codex_home(std::path::Path::new(&codex_home))
}

pub fn preview_provider_migration(request: MigrationRequest) -> Result<MigrationResult> {
    migration::preview_provider_migration(request)
}

pub fn apply_provider_migration(request: MigrationRequest) -> Result<MigrationResult> {
    migration::apply_provider_migration(request)
}
```

- [ ] **Step 5: Register Tauri commands through wrappers**

Modify `apps/desktop/src-tauri/src/main.rs`:

```rust
use ai_session_migrator::codex::{self, CommandError, MigrationRequest, MigrationResult, ScanResponse};

#[tauri::command]
fn app_health() -> &'static str {
    "ok"
}

#[tauri::command]
fn scan_codex_home(codex_home: String) -> std::result::Result<ScanResponse, CommandError> {
    codex::scan_codex_home(codex_home)
}

#[tauri::command]
fn preview_provider_migration(request: MigrationRequest) -> std::result::Result<MigrationResult, CommandError> {
    codex::preview_provider_migration(request)
}

#[tauri::command]
fn apply_provider_migration(request: MigrationRequest) -> std::result::Result<MigrationResult, CommandError> {
    codex::apply_provider_migration(request)
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            app_health,
            scan_codex_home,
            preview_provider_migration,
            apply_provider_migration
        ])
        .run(tauri::generate_context!())
        .expect("failed to run AI Session Migrator");
}
```

- [ ] **Step 6: Run Rust contract check**

Run:

```powershell
cargo test --lib
```

Expected: compilation fails at this point because modules `backup`, `metadata`, `migration`, `scan`, and `sqlite` are declared but not created. This is the intended failing checkpoint before adding modules.

- [ ] **Step 7: Commit contracts after empty modules compile**

After Task 2 creates module files and `cargo test --lib` compiles, commit this task and Task 2 together:

```powershell
git add apps/desktop/src-tauri/Cargo.toml apps/desktop/src-tauri/src/lib.rs apps/desktop/src-tauri/src/main.rs apps/desktop/src-tauri/src/codex
git commit -m "feat: add Rust command contracts"
```

## Task 2: Metadata Parsing and Test Fixtures

**Files:**
- Create: `apps/desktop/src-tauri/src/codex/metadata.rs`
- Create: `apps/desktop/src-tauri/src/codex/test_support.rs`
- Create: `apps/desktop/src-tauri/src/codex/backup.rs`
- Create: `apps/desktop/src-tauri/src/codex/sqlite.rs`
- Create: `apps/desktop/src-tauri/src/codex/scan.rs`
- Create: `apps/desktop/src-tauri/src/codex/migration.rs`

- [ ] **Step 1: Add minimal empty modules**

Create these files with the shown content so the module tree compiles before detailed tests:

`apps/desktop/src-tauri/src/codex/backup.rs`

```rust
use crate::codex::{CommandError, Result};
use std::path::{Path, PathBuf};

pub fn create_backup_dir(_codex_home: &Path, _files: &[PathBuf]) -> Result<PathBuf> {
    Err(CommandError::new("not_ready", "backup support is not ready"))
}
```

`apps/desktop/src-tauri/src/codex/sqlite.rs`

```rust
use crate::codex::metadata::SessionMetadata;
use crate::codex::Result;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateEntry {
    pub db_path: String,
    pub exists: bool,
    pub provider: Option<String>,
    pub archived: Option<i32>,
}

pub fn state_dbs(_codex_home: &Path) -> Vec<std::path::PathBuf> {
    Vec::new()
}

pub fn state_entry(_db: &Path, _thread_id: &str) -> StateEntry {
    StateEntry {
        db_path: String::new(),
        exists: false,
        provider: None,
        archived: None,
    }
}

pub fn upsert_state_entries(_codex_home: &Path, _items: &[SessionMetadata], _provider: &str) -> Result<()> {
    Ok(())
}
```

`apps/desktop/src-tauri/src/codex/scan.rs`

```rust
use crate::codex::{CommandError, Result, ScanResponse};
use std::path::Path;

pub fn scan_codex_home(_codex_home: &Path) -> Result<ScanResponse> {
    Err(CommandError::new("not_ready", "scan support is not ready"))
}
```

`apps/desktop/src-tauri/src/codex/migration.rs`

```rust
use crate::codex::{CommandError, MigrationRequest, MigrationResult, Result};

pub fn preview_provider_migration(_request: MigrationRequest) -> Result<MigrationResult> {
    Err(CommandError::new("not_ready", "migration preview is not ready"))
}

pub fn apply_provider_migration(_request: MigrationRequest) -> Result<MigrationResult> {
    Err(CommandError::new("not_ready", "migration apply is not ready"))
}
```

- [ ] **Step 2: Write metadata tests first**

Create `apps/desktop/src-tauri/src/codex/metadata.rs` with the tests and a small failing skeleton:

```rust
use crate::codex::{CommandError, Result};
use regex::bytes::Regex;
use serde_json::Value;
use std::path::{Path, PathBuf};

pub const BOM: &[u8] = b"\xef\xbb\xbf";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionMetadata {
    pub thread_id: String,
    pub provider: Option<String>,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
    pub cwd: String,
    pub source: String,
    pub cli_version: String,
    pub thread_source: Option<String>,
    pub title: String,
    pub first_user_message: String,
    pub preview: String,
    pub path: PathBuf,
}

pub fn metadata_from_bytes(raw: &[u8], path: &Path) -> Result<SessionMetadata> {
    let _ = (raw, path);
    Err(CommandError::new("not_ready", "metadata parsing is not ready"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_jsonl(thread_id: &str, provider: &str, message: &str) -> Vec<u8> {
        let text = format!(
            "{{\"timestamp\":\"2026-06-15T01:00:00.000Z\",\"type\":\"session_meta\",\"payload\":{{\"id\":\"{thread_id}\",\"timestamp\":\"2026-06-15T01:00:00.000Z\",\"cwd\":\"D:\\\\work\",\"source\":\"vscode\",\"model_provider\":\"{provider}\",\"cli_version\":\"0.140.0\",\"thread_source\":\"user\"}}}}\n{{\"timestamp\":\"2026-06-15T01:01:00.000Z\",\"type\":\"response_item\",\"payload\":{{\"type\":\"message\",\"role\":\"user\",\"content\":[{{\"type\":\"input_text\",\"text\":\"{message}\"}}]}}}}\n"
        );
        text.into_bytes()
    }

    #[test]
    fn parses_metadata_and_preserves_chinese_preview() {
        let raw = sample_jsonl(
            "019eca3b-941d-7340-9b14-328c635a6523",
            "funai",
            "你好，迁移 provider",
        );

        let metadata = metadata_from_bytes(&raw, Path::new("rollout.jsonl")).unwrap();

        assert_eq!(metadata.thread_id, "019eca3b-941d-7340-9b14-328c635a6523");
        assert_eq!(metadata.provider.as_deref(), Some("funai"));
        assert_eq!(metadata.title, "你好，迁移 provider");
        assert_eq!(metadata.preview, "你好，迁移 provider");
        assert_eq!(metadata.created_at_ms, 1_781_484_400_000);
        assert_eq!(metadata.updated_at_ms, 1_781_484_460_000);
    }

    #[test]
    fn accepts_utf8_bom_before_first_json_line() {
        let mut raw = BOM.to_vec();
        raw.extend(sample_jsonl(
            "019ec94d-720d-7a12-a379-28c8042bc6b4",
            "gmn",
            "带 BOM 的会话",
        ));

        let metadata = metadata_from_bytes(&raw, Path::new("rollout.jsonl")).unwrap();

        assert_eq!(metadata.thread_id, "019ec94d-720d-7a12-a379-28c8042bc6b4");
        assert_eq!(metadata.provider.as_deref(), Some("gmn"));
        assert_eq!(metadata.preview, "带 BOM 的会话");
    }
}
```

- [ ] **Step 3: Run metadata tests and verify failure**

Run:

```powershell
cargo test metadata::tests --lib
```

Expected: both tests fail with `not_ready`.

- [ ] **Step 4: Implement metadata parsing**

Replace `metadata_from_bytes` in `metadata.rs` with working code:

```rust
pub fn metadata_from_bytes(raw: &[u8], path: &Path) -> Result<SessionMetadata> {
    let without_bom = if raw.starts_with(BOM) { &raw[BOM.len()..] } else { raw };
    let text = std::str::from_utf8(without_bom).map_err(|error| {
        CommandError::new(
            "invalid_utf8",
            format!("{} is not valid UTF-8 ({error})", path.display()),
        )
    })?;
    let lines: Vec<&str> = text.lines().filter(|line| !line.trim().is_empty()).collect();
    let first_line = lines.first().ok_or_else(|| {
        CommandError::new("empty_session", format!("{} is empty", path.display()))
    })?;
    let first: Value = serde_json::from_str(first_line).map_err(|error| {
        CommandError::new(
            "invalid_jsonl",
            format!("{} has invalid session metadata JSON ({error})", path.display()),
        )
    })?;
    if first.get("type").and_then(Value::as_str) != Some("session_meta") {
        return Err(CommandError::new(
            "missing_session_meta",
            format!("{} does not start with session metadata", path.display()),
        ));
    }
    let payload = first.get("payload").and_then(Value::as_object).ok_or_else(|| {
        CommandError::new("missing_payload", format!("{} session metadata has no payload", path.display()))
    })?;
    let thread_id = payload
        .get("id")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| CommandError::new("missing_thread_id", format!("{} session metadata has no id", path.display())))?
        .to_string();

    let mut first_user = String::new();
    let mut last_timestamp = first
        .get("timestamp")
        .and_then(Value::as_str)
        .or_else(|| payload.get("timestamp").and_then(Value::as_str))
        .map(str::to_string);

    for line in lines.iter().skip(1) {
        let value: Value = serde_json::from_str(line).map_err(|error| {
            CommandError::new("invalid_jsonl", format!("{} has invalid JSONL row ({error})", path.display()))
        })?;
        if let Some(timestamp) = value.get("timestamp").and_then(Value::as_str) {
            last_timestamp = Some(timestamp.to_string());
        }
        if first_user.is_empty() {
            first_user = extract_user_text(value.get("payload").unwrap_or(&Value::Null));
            if first_user.starts_with("<environment_context>") {
                first_user.clear();
            }
        }
    }

    let preview = clean_preview(&first_user).unwrap_or_else(|| thread_id.clone());
    let title = title_from_preview(&preview);
    let created_at_ms = timestamp_ms(
        payload
            .get("timestamp")
            .and_then(Value::as_str)
            .or_else(|| first.get("timestamp").and_then(Value::as_str)),
    );
    let updated_at_ms = timestamp_ms(last_timestamp.as_deref());
    let provider = provider_from_bytes(without_bom).or_else(|| {
        payload
            .get("model_provider")
            .and_then(Value::as_str)
            .map(str::to_string)
    });

    Ok(SessionMetadata {
        thread_id,
        provider,
        created_at_ms,
        updated_at_ms,
        cwd: payload.get("cwd").and_then(Value::as_str).unwrap_or_default().to_string(),
        source: payload.get("source").and_then(Value::as_str).unwrap_or("vscode").to_string(),
        cli_version: payload.get("cli_version").and_then(Value::as_str).unwrap_or_default().to_string(),
        thread_source: payload.get("thread_source").and_then(Value::as_str).map(str::to_string),
        title,
        first_user_message: preview.clone(),
        preview,
        path: path.to_path_buf(),
    })
}

fn provider_from_bytes(raw: &[u8]) -> Option<String> {
    let search_len = raw.len().min(20_000);
    let pattern = Regex::new(r#""model_provider"\s*:\s*"([^"]+)""#).ok()?;
    let captures = pattern.captures(&raw[..search_len])?;
    std::str::from_utf8(captures.get(1)?.as_bytes()).ok().map(str::to_string)
}

fn extract_user_text(payload: &Value) -> String {
    if payload.get("type").and_then(Value::as_str) == Some("user_message") {
        return payload.get("message").and_then(Value::as_str).unwrap_or_default().to_string();
    }
    if payload.get("type").and_then(Value::as_str) == Some("message")
        && payload.get("role").and_then(Value::as_str) == Some("user")
    {
        let mut parts = Vec::new();
        if let Some(content) = payload.get("content").and_then(Value::as_array) {
            for item in content {
                if item.get("type").and_then(Value::as_str) == Some("input_text") {
                    parts.push(item.get("text").and_then(Value::as_str).unwrap_or_default());
                }
            }
        }
        return parts.join("");
    }
    String::new()
}

fn clean_preview(text: &str) -> Option<String> {
    let mut value = text.trim().to_string();
    let marker = "## My request for Codex:";
    if let Some((_, rest)) = value.split_once(marker) {
        value = rest.trim().to_string();
    }
    let lines: Vec<&str> = value
        .lines()
        .filter(|line| !line.starts_with("<image "))
        .collect();
    let cleaned = lines.join("\n").trim().chars().take(500).collect::<String>();
    if cleaned.is_empty() { None } else { Some(cleaned) }
}

fn title_from_preview(preview: &str) -> String {
    preview
        .lines()
        .next()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("Untitled session")
        .chars()
        .take(80)
        .collect()
}

fn timestamp_ms(value: Option<&str>) -> i64 {
    let Some(value) = value else {
        return chrono::Utc::now().timestamp_millis();
    };
    let normalized = value.replace('Z', "+00:00");
    chrono::DateTime::parse_from_rfc3339(&normalized)
        .map(|timestamp| timestamp.timestamp_millis())
        .unwrap_or_else(|_| chrono::Utc::now().timestamp_millis())
}
```

- [ ] **Step 5: Run metadata tests and verify pass**

Run:

```powershell
cargo test metadata::tests --lib
```

Expected: both metadata tests pass.

- [ ] **Step 6: Add reusable test support**

Create `apps/desktop/src-tauri/src/codex/test_support.rs` with fixture helpers:

```rust
use crate::codex::metadata::BOM;
use rusqlite::Connection;
use std::fs;
use std::path::Path;

pub fn write_jsonl(path: &Path, thread_id: &str, provider: &str, bom: bool, message: &str) {
    let data = format!(
        "{{\"timestamp\":\"2026-06-15T01:00:00.000Z\",\"type\":\"session_meta\",\"payload\":{{\"id\":\"{thread_id}\",\"timestamp\":\"2026-06-15T01:00:00.000Z\",\"cwd\":\"D:\\\\work\",\"source\":\"vscode\",\"model_provider\":\"{provider}\",\"cli_version\":\"0.140.0\",\"thread_source\":\"user\"}}}}\n{{\"timestamp\":\"2026-06-15T01:01:00.000Z\",\"type\":\"response_item\",\"payload\":{{\"type\":\"message\",\"role\":\"user\",\"content\":[{{\"type\":\"input_text\",\"text\":\"{message}\"}}]}}}}\n"
    );
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    let mut raw = Vec::new();
    if bom {
        raw.extend(BOM);
    }
    raw.extend(data.as_bytes());
    fs::write(path, raw).unwrap();
}

pub fn init_state_db(path: &Path) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    let connection = Connection::open(path).unwrap();
    connection
        .execute_batch(
            "
            create table threads (
                id text primary key,
                rollout_path text not null,
                created_at integer not null,
                updated_at integer not null,
                source text not null,
                model_provider text not null,
                cwd text not null,
                title text not null,
                sandbox_policy text not null,
                approval_mode text not null,
                tokens_used integer not null default 0,
                has_user_event integer not null default 0,
                archived integer not null default 0,
                archived_at integer,
                git_sha text,
                git_branch text,
                git_origin_url text,
                cli_version text not null default '',
                first_user_message text not null default '',
                agent_nickname text,
                agent_role text,
                memory_mode text not null default 'enabled',
                model text,
                reasoning_effort text,
                agent_path text,
                created_at_ms integer,
                updated_at_ms integer,
                thread_source text,
                preview text not null default ''
            );
            ",
        )
        .unwrap();
}

pub fn insert_state_row(path: &Path, thread_id: &str, provider: &str, archived: i32) {
    let connection = Connection::open(path).unwrap();
    connection
        .execute(
            "insert into threads (
                id, rollout_path, created_at, updated_at, source, model_provider, cwd, title,
                sandbox_policy, approval_mode, tokens_used, has_user_event, archived, archived_at,
                cli_version, first_user_message, memory_mode, created_at_ms, updated_at_ms, preview
             ) values (?1, '', 0, 0, 'vscode', ?2, '', 'old title', '{}', 'never', 0, 1, ?3, NULL, '', '', 'enabled', 0, 0, '')",
            (thread_id, provider, archived),
        )
        .unwrap();
}
```

- [ ] **Step 7: Run library tests**

Run:

```powershell
cargo test --lib
```

Expected: all current library tests pass.

## Task 3: Scan Dashboard and Provider Options

**Files:**
- Modify: `apps/desktop/src-tauri/src/codex/scan.rs`
- Modify: `apps/desktop/src-tauri/src/codex/sqlite.rs`

- [ ] **Step 1: Write scan tests**

Replace `apps/desktop/src-tauri/src/codex/scan.rs` with tests and a failing function:

```rust
use crate::codex::metadata::metadata_from_bytes;
use crate::codex::sqlite::{state_dbs, state_entry};
use crate::codex::{
    CommandError, DashboardModel, ProviderOption, ProviderOptionKind, ProviderOptions, Result,
    ScanResponse, ThreadRow,
};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

pub fn scan_codex_home(_codex_home: &Path) -> Result<ScanResponse> {
    Err(CommandError::new("not_ready", "scan support is not ready"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codex::test_support::{init_state_db, write_jsonl};

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
        fs::write(codex.join("config.toml"), "model_provider = \"yihubangg\"\n").unwrap();
        fs::write(codex.join("session_index.jsonl"), "").unwrap();
        init_state_db(&codex.join("state_5.sqlite"));

        let response = scan_codex_home(&codex).unwrap();

        assert_eq!(response.config_provider.as_deref(), Some("yihubangg"));
        assert_eq!(response.dashboard.total_threads, 2);
        assert_eq!(response.dashboard.problem_threads, 2);
        assert!(response.provider_options.source_providers.contains(&"funai".to_string()));
        assert!(response.provider_options.source_providers.contains(&"gmn".to_string()));
        assert_eq!(response.provider_options.target_providers[0].value, "yihubangg");
        assert!(response.provider_options.target_providers[0].recommended);
        let first = response.dashboard.rows.iter().find(|row| row.thread_id == wanted).unwrap();
        assert!(first.issue_codes.contains(&"bom_present".to_string()));
        assert!(first.issue_codes.contains(&"provider_mismatch".to_string()));
        assert!(first.issue_codes.contains(&"missing_index".to_string()));
        assert!(first.issue_codes.contains(&"missing_state_entry".to_string()));
        assert_eq!(first.display_name, "你好，迁移 provider");
    }
}
```

- [ ] **Step 2: Run scan test and verify failure**

Run:

```powershell
cargo test scan::tests::scan_builds_dashboard_and_provider_options --lib
```

Expected: fails with `not_ready`.

- [ ] **Step 3: Implement sqlite read helpers**

Replace `apps/desktop/src-tauri/src/codex/sqlite.rs` with:

```rust
use crate::codex::metadata::SessionMetadata;
use crate::codex::{CommandError, Result};
use rusqlite::{Connection, OptionalExtension};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateEntry {
    pub db_path: String,
    pub exists: bool,
    pub provider: Option<String>,
    pub archived: Option<i32>,
}

pub fn state_dbs(codex_home: &Path) -> Vec<PathBuf> {
    [codex_home.join("state_5.sqlite"), codex_home.join("sqlite/state_5.sqlite")]
        .into_iter()
        .filter(|path| path.exists())
        .collect()
}

pub fn state_entry(db: &Path, thread_id: &str) -> StateEntry {
    let db_path = db.display().to_string();
    let Ok(connection) = Connection::open_with_flags(db, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY) else {
        return StateEntry { db_path, exists: false, provider: None, archived: None };
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
        _ => StateEntry { db_path, exists: false, provider: None, archived: None },
    }
}

pub fn upsert_state_entries(_codex_home: &Path, _items: &[SessionMetadata], _provider: &str) -> Result<()> {
    Err(CommandError::new("not_ready", "sqlite write support is not ready"))
}
```

- [ ] **Step 4: Implement scan logic**

Replace the body of `scan_codex_home` and add helper functions in `scan.rs`:

```rust
pub fn scan_codex_home(codex_home: &Path) -> Result<ScanResponse> {
    if !codex_home.exists() {
        return Err(CommandError::new("codex_home_missing", format!("Codex home does not exist: {}", codex_home.display())));
    }
    let sessions_dir = codex_home.join("sessions");
    if !sessions_dir.exists() {
        return Err(CommandError::new("sessions_missing", format!("sessions directory does not exist: {}", sessions_dir.display())));
    }

    let config_provider = config_provider(codex_home)?;
    let index_ids = index_ids(codex_home)?;
    let dbs = state_dbs(codex_home);
    let mut rows = Vec::new();
    let mut source_providers = BTreeSet::new();

    for path in session_files(&sessions_dir)? {
        let raw = fs::read(&path).map_err(|error| CommandError::io("read session", path.display(), error))?;
        let metadata = metadata_from_bytes(&raw, &path)?;
        if let Some(provider) = &metadata.provider {
            source_providers.insert(provider.clone());
        }
        let mut issue_codes = Vec::new();
        if raw.starts_with(crate::codex::metadata::BOM) {
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
        for db in &dbs {
            let entry = state_entry(db, &metadata.thread_id);
            if !entry.exists {
                issue_codes.push("missing_state_entry".to_string());
            } else {
                if let (Some(state_provider), Some(file_provider)) = (&entry.provider, &metadata.provider) {
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
        rows.push(thread_row(codex_home, &metadata, config_provider.clone(), issue_codes));
    }

    rows.sort_by(|left, right| right.severity.cmp(&left.severity).then_with(|| left.thread_id.cmp(&right.thread_id)));
    let mut issue_counts = BTreeMap::new();
    for row in &rows {
        for code in &row.issue_codes {
            *issue_counts.entry(code.clone()).or_insert(0) += 1;
        }
    }
    let dashboard = DashboardModel {
        codex_home: codex_home.display().to_string(),
        total_threads: rows.len(),
        problem_threads: rows.iter().filter(|row| !row.issue_codes.is_empty()).count(),
        issue_counts,
        rows,
    };
    let provider_options = provider_options(config_provider.clone(), source_providers);
    Ok(ScanResponse { dashboard, provider_options, config_provider })
}
```

Also add helpers `session_files`, `config_provider`, `index_ids`, `thread_row`, `severity`, `suggested_action_code`, `suggested_action_values`, and `provider_options` in the same file. Use the same issue severity order from `ai_session_doctor/ui_model.py`: `bom_present=100`, `missing_state_entry=90`, `state_provider_mismatch=80`, `provider_mismatch=70`, `missing_index=60`, `archived_state=50`.

- [ ] **Step 5: Run scan tests**

Run:

```powershell
cargo test scan::tests --lib
```

Expected: scan test passes.

- [ ] **Step 6: Commit scan support**

Run:

```powershell
git add apps/desktop/src-tauri
git commit -m "feat: scan Codex sessions in Rust"
```

## Task 4: Migration Preview and Apply

**Files:**
- Modify: `apps/desktop/src-tauri/src/codex/backup.rs`
- Modify: `apps/desktop/src-tauri/src/codex/sqlite.rs`
- Modify: `apps/desktop/src-tauri/src/codex/migration.rs`

- [ ] **Step 1: Write migration tests**

Replace `migration.rs` tests with coverage for preview and apply:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::codex::test_support::{init_state_db, write_jsonl};
    use rusqlite::Connection;
    use std::fs;

    #[test]
    fn preview_returns_changed_threads_without_writes() {
        let temp = tempfile::tempdir().unwrap();
        let codex = temp.path().join(".codex");
        let thread_id = "019eca3b-941d-7340-9b14-328c635a6523";
        let rollout = codex.join("sessions/2026/06/15/rollout-a-019eca3b-941d-7340-9b14-328c635a6523.jsonl");
        write_jsonl(&rollout, thread_id, "funai", false, "你好");
        let before = fs::read(&rollout).unwrap();

        let result = preview_provider_migration(MigrationRequest {
            codex_home: codex.display().to_string(),
            source_provider: Some("funai".to_string()),
            target_provider: "yihubangg".to_string(),
            thread_ids: vec![thread_id.to_string()],
        }).unwrap();

        assert_eq!(result.changed_threads, vec![thread_id.to_string()]);
        assert!(result.dry_run);
        assert_eq!(fs::read(&rollout).unwrap(), before);
    }

    #[test]
    fn apply_changes_provider_creates_backup_and_repairs_visibility() {
        let temp = tempfile::tempdir().unwrap();
        let codex = temp.path().join(".codex");
        let thread_id = "019ec94d-720d-7a12-a379-28c8042bc6b4";
        let rollout = codex.join("sessions/2026/06/15/rollout-a-019ec94d-720d-7a12-a379-28c8042bc6b4.jsonl");
        write_jsonl(&rollout, thread_id, "funai", true, "你好，保留中文");
        fs::write(codex.join("session_index.jsonl"), "").unwrap();
        let db = codex.join("sqlite/state_5.sqlite");
        init_state_db(&db);

        let result = apply_provider_migration(MigrationRequest {
            codex_home: codex.display().to_string(),
            source_provider: Some("funai".to_string()),
            target_provider: "yihubangg".to_string(),
            thread_ids: vec![thread_id.to_string()],
        }).unwrap();

        assert_eq!(result.changed_threads, vec![thread_id.to_string()]);
        assert!(!result.dry_run);
        let backup_dir = result.backup_dir.unwrap();
        assert!(std::path::Path::new(&backup_dir).exists());
        let raw = fs::read(&rollout).unwrap();
        assert!(!raw.starts_with(crate::codex::metadata::BOM));
        let text = String::from_utf8(raw).unwrap();
        assert!(text.contains("\"model_provider\":\"yihubangg\""));
        assert!(text.contains("你好，保留中文"));
        assert!(fs::read_to_string(codex.join("session_index.jsonl")).unwrap().contains(thread_id));
        let connection = Connection::open(&db).unwrap();
        let row: (String, i32, String) = connection
            .query_row("select model_provider, archived, preview from threads where id=?1", [thread_id], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?))
            })
            .unwrap();
        assert_eq!(row.0, "yihubangg");
        assert_eq!(row.1, 0);
        assert!(row.2.contains("你好"));
    }
}
```

- [ ] **Step 2: Run migration tests and verify failure**

Run:

```powershell
cargo test migration::tests --lib
```

Expected: migration tests fail with `not_ready`.

- [ ] **Step 3: Implement backup creation**

Replace `backup.rs` with:

```rust
use crate::codex::{CommandError, Result};
use chrono::Local;
use std::fs;
use std::path::{Path, PathBuf};

pub fn create_backup_dir(codex_home: &Path, files: &[PathBuf]) -> Result<PathBuf> {
    let stamp = Local::now().format("%Y%m%d-%H%M%S").to_string();
    let backup_dir = codex_home.join(format!("ai-session-migrator-backup-{stamp}"));
    fs::create_dir_all(&backup_dir)
        .map_err(|error| CommandError::io("create backup directory", backup_dir.display(), error))?;
    for path in files {
        if path.exists() {
            let file_name = path.file_name().ok_or_else(|| {
                CommandError::new("invalid_backup_file", format!("Cannot back up path without file name: {}", path.display()))
            })?;
            fs::copy(path, backup_dir.join(file_name))
                .map_err(|error| CommandError::io("copy backup file", path.display(), error))?;
        }
    }
    Ok(backup_dir)
}
```

- [ ] **Step 4: Implement sqlite writes**

Replace `upsert_state_entries` in `sqlite.rs` with logic that opens both known state DB paths, sets `pragma busy_timeout=5000`, and inserts/updates rows using the same columns as the test fixture. Use `metadata.created_at_ms`, `metadata.updated_at_ms`, `metadata.title`, `metadata.preview`, `metadata.first_user_message`, `metadata.cwd`, `metadata.source`, `metadata.cli_version`, and `metadata.thread_source`.

- [ ] **Step 5: Implement migration preview/apply**

Implement `preview_provider_migration` and `apply_provider_migration` in `migration.rs`:

- Validate `target_provider.trim()` is non-empty.
- Validate `thread_ids` is non-empty.
- Scan `sessions/**/rollout-*.jsonl`.
- Keep only selected thread IDs.
- When `source_provider` is present, keep only matching source provider.
- For preview, return `MigrationResult { changed_threads, planned_repairs, backup_dir: None, dry_run: true }`.
- For apply, create one backup containing changed rollout files, `session_index.jsonl` when present, and all sqlite DBs when present.
- Write changed rollout bytes after removing BOM and replacing the first provider marker with compact `"model_provider":"target"`.
- Ensure index rows for selected metadata.
- Upsert sqlite rows and clear archived state.

- [ ] **Step 6: Run migration tests**

Run:

```powershell
cargo test migration::tests --lib
```

Expected: preview and apply tests pass.

- [ ] **Step 7: Run all Rust library tests**

Run:

```powershell
cargo test --lib
```

Expected: all Rust library tests pass.

- [ ] **Step 8: Commit migration core**

Run:

```powershell
git add apps/desktop/src-tauri
git commit -m "feat: migrate Codex providers in Rust"
```

## Task 5: TypeScript Command Adapter

**Files:**
- Modify: `apps/desktop/src/domain/session.ts`
- Create: `apps/desktop/src/domain/migrationApi.ts`

- [ ] **Step 1: Replace TypeScript domain types**

Replace `apps/desktop/src/domain/session.ts` with TypeScript contracts matching Rust:

```ts
export type ProviderOptionKind = "config" | "discovered";

export type ThreadRow = {
  threadId: string;
  shortId: string;
  displayName: string;
  path: string;
  fileProvider: string | null;
  configProvider: string | null;
  issueCodes: string[];
  severity: number;
  canMigrate: boolean;
  suggestedActionCode: string;
  suggestedActionValues: Record<string, string>;
  updatedAtMs: number;
};

export type DashboardModel = {
  codexHome: string;
  totalThreads: number;
  problemThreads: number;
  issueCounts: Record<string, number>;
  rows: ThreadRow[];
};

export type ProviderOption = {
  value: string;
  label: string;
  kind: ProviderOptionKind;
  recommended: boolean;
};

export type ProviderOptions = {
  currentConfigProvider: string | null;
  sourceProviders: string[];
  targetProviders: ProviderOption[];
};

export type ScanResponse = {
  dashboard: DashboardModel;
  providerOptions: ProviderOptions;
  configProvider: string | null;
};

export type PlannedRepair = {
  threadId: string;
  code: string;
  message: string;
};

export type MigrationRequest = {
  codexHome: string;
  sourceProvider: string | null;
  targetProvider: string;
  threadIds: string[];
};

export type MigrationResult = {
  changedThreads: string[];
  plannedRepairs: PlannedRepair[];
  backupDir: string | null;
  dryRun: boolean;
};

export type CommandError = {
  code: string;
  message: string;
};
```

- [ ] **Step 2: Add Tauri API adapter**

Create `apps/desktop/src/domain/migrationApi.ts`:

```ts
import { invoke } from "@tauri-apps/api/core";
import type { MigrationRequest, MigrationResult, ScanResponse } from "./session";

export type MigrationApi = {
  scanCodexHome(codexHome: string): Promise<ScanResponse>;
  previewProviderMigration(request: MigrationRequest): Promise<MigrationResult>;
  applyProviderMigration(request: MigrationRequest): Promise<MigrationResult>;
};

export const tauriMigrationApi: MigrationApi = {
  scanCodexHome(codexHome) {
    return invoke<ScanResponse>("scan_codex_home", { codexHome });
  },
  previewProviderMigration(request) {
    return invoke<MigrationResult>("preview_provider_migration", { request });
  },
  applyProviderMigration(request) {
    return invoke<MigrationResult>("apply_provider_migration", { request });
  }
};
```

- [ ] **Step 3: Run TypeScript build and verify expected App errors**

Run:

```powershell
npm run build
```

Expected: TypeScript fails because `App.tsx` still imports `mockSessions`. This is the intended checkpoint before replacing the UI.

- [ ] **Step 4: Commit adapter after App compiles in Task 6**

Commit this task with Task 6 after the UI is updated:

```powershell
git add apps/desktop/src/domain apps/desktop/src/App.tsx apps/desktop/src/App.test.tsx apps/desktop/src/styles.css
git commit -m "feat: connect React workflow to migration API"
```

## Task 6: React Workflow UI

**Files:**
- Modify: `apps/desktop/src/App.tsx`
- Modify: `apps/desktop/src/App.test.tsx`
- Modify: `apps/desktop/src/styles.css`

- [ ] **Step 1: Write UI tests against fake API**

Replace `apps/desktop/src/App.test.tsx` with tests that pass a fake API into `<App migrationApi={fakeApi} />`. The fake scan response must include:

```ts
const scanResponse = {
  dashboard: {
    codexHome: "C:\\Users\\jianrui\\.codex",
    totalThreads: 2,
    problemThreads: 2,
    issueCounts: { provider_mismatch: 1, missing_index: 1 },
    rows: [
      {
        threadId: "019eca3b-941d-7340-9b14-328c635a6523",
        shortId: "019eca3b",
        displayName: "恢复 provider 切换后的会话",
        path: "C:\\Users\\jianrui\\.codex\\sessions\\rollout-a.jsonl",
        fileProvider: "funai",
        configProvider: "yihubangg",
        issueCodes: ["provider_mismatch", "missing_index"],
        severity: 70,
        canMigrate: true,
        suggestedActionCode: "migrate_provider",
        suggestedActionValues: { source: "funai", target: "yihubangg" },
        updatedAtMs: 1781484460000
      },
      {
        threadId: "019ec94d-720d-7a12-a379-28c8042bc6b4",
        shortId: "019ec94d",
        displayName: "另一个 provider 会话",
        path: "C:\\Users\\jianrui\\.codex\\sessions\\rollout-b.jsonl",
        fileProvider: "gmn",
        configProvider: "yihubangg",
        issueCodes: ["provider_mismatch"],
        severity: 70,
        canMigrate: true,
        suggestedActionCode: "migrate_provider",
        suggestedActionValues: { source: "gmn", target: "yihubangg" },
        updatedAtMs: 1781484400000
      }
    ]
  },
  providerOptions: {
    currentConfigProvider: "yihubangg",
    sourceProviders: ["funai", "gmn"],
    targetProviders: [
      { value: "yihubangg", label: "yihubangg（当前配置，推荐）", kind: "config", recommended: true },
      { value: "funai", label: "funai", kind: "discovered", recommended: false },
      { value: "gmn", label: "gmn", kind: "discovered", recommended: false }
    ]
  },
  configProvider: "yihubangg"
};
```

Required tests:

- Renders Chinese workflow controls and no mojibake headings.
- Scan populates source and target provider dropdowns.
- Source provider filter hides non-matching rows.
- Selecting custom target provider reveals custom input.
- Preview calls fake API with selected thread IDs and target provider.
- Apply result shows changed thread IDs and backup path.

- [ ] **Step 2: Run UI tests and verify failure**

Run:

```powershell
npm test
```

Expected: tests fail because `App` does not accept `migrationApi` and still uses prototype data.

- [ ] **Step 3: Replace App with real state machine**

Modify `apps/desktop/src/App.tsx` so:

- `App` accepts optional prop `{ migrationApi?: MigrationApi }`.
- Default API is `tauriMigrationApi`.
- State includes `codexHome`, `scanResponse`, `sourceProvider`, `targetChoice`, `customTargetProvider`, `selectedIds`, `expandedIds`, `previewResult`, `applyResult`, `loading`, and `error`.
- `handleScan` calls `migrationApi.scanCodexHome(codexHome)`, saves response, defaults `targetChoice` to current config provider when present, and selects all migratable rows.
- `visibleRows` filters by source provider unless source provider is `"__all__"`.
- `resolvedTargetProvider` is custom text when target choice is `"__custom__"`, otherwise the selected target choice.
- `handlePreview` calls `previewProviderMigration`.
- `handleApply` calls `applyProviderMigration`.
- Buttons disable when target provider or selected sessions are missing.
- All user-facing copy is real Chinese.

- [ ] **Step 4: Update CSS for form controls and result panels**

Modify `styles.css` so `select`, `.result-panel`, `.error-panel`, `.metrics`, `.provider-select`, and disabled buttons are styled consistently with the existing sober desktop theme. Keep card radii at `8px` or below.

- [ ] **Step 5: Run UI tests**

Run:

```powershell
npm test
```

Expected: all frontend tests pass.

- [ ] **Step 6: Run frontend build**

Run:

```powershell
npm run build
```

Expected: TypeScript and Vite build pass.

- [ ] **Step 7: Commit UI integration**

Run:

```powershell
git add apps/desktop/src/domain apps/desktop/src/App.tsx apps/desktop/src/App.test.tsx apps/desktop/src/styles.css
git commit -m "feat: connect React workflow to migration API"
```

## Task 7: Full Verification and Desktop Smoke

**Files:**
- Modify only if verification finds defects.

- [ ] **Step 1: Run Rust checks**

Run:

```powershell
cargo test --lib
```

Expected: all Rust library tests pass.

Run:

```powershell
cargo check
```

Expected: Tauri crate checks successfully. If dependency compilation takes longer than two minutes, rerun with a longer timeout before diagnosing code.

- [ ] **Step 2: Run frontend checks**

Run:

```powershell
npm test
npm run build
```

Expected: Vitest and frontend build pass.

- [ ] **Step 3: Start local Vite app**

Run:

```powershell
npm run dev
```

Expected: Vite serves the app on `http://127.0.0.1:5173` or the next available port.

- [ ] **Step 4: Browser smoke test**

Open the Vite URL in the in-app Browser and verify:

- Heading is Chinese and readable.
- Codex path field is visible.
- Scan, source provider dropdown, target provider dropdown, custom target provider input path, preview, and confirm controls render without overlap.
- No mojibake appears in primary UI copy.
- Buttons show disabled state before required inputs are available.

- [ ] **Step 5: Commit verification fixes**

If smoke verification requires fixes:

```powershell
git add apps/desktop/src apps/desktop/src-tauri
git commit -m "fix: polish provider migration workflow"
```

If no fixes are needed, do not create an empty commit.

## Self-Review Checklist

- Spec coverage: Tasks cover Rust-native scanning, provider option aggregation, dry-run preview, apply with backups, index/sqlite repair, React provider dropdowns, custom target provider, result reporting, and verification.
- Safety: Apply is the only write path, and Task 4 requires backups before writes.
- Type consistency: Rust and TypeScript both use camelCase fields: `providerOptions`, `currentConfigProvider`, `sourceProviders`, `targetProviders`, `threadIds`, `changedThreads`, `backupDir`, and `dryRun`.
- Scope: This plan stays on the narrow provider migration workflow and does not build a full repair console.

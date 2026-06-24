pub mod archive;
pub mod backup;
pub mod deletion;
pub mod error;
pub mod metadata;
pub mod migration;
pub mod restart;
pub mod scan;
pub mod sqlite;

#[cfg(test)]
pub mod test_support;

pub use error::{CommandError, Result};
pub use restart::{ProviderRestartRequest, ProviderRestartResult};

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ThreadRow {
    pub thread_id: String,
    pub short_id: String,
    pub display_name: String,
    pub path: String,
    pub file_provider: Option<String>,
    pub config_provider: Option<String>,
    pub lifecycle: ThreadLifecycle,
    pub issue_codes: Vec<String>,
    pub severity: i32,
    pub can_migrate: bool,
    pub suggested_action_code: String,
    pub suggested_action_values: BTreeMap<String, String>,
    pub updated_at_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ThreadLifecycle {
    Active,
    Archived,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DashboardModel {
    pub codex_home: String,
    pub total_threads: usize,
    pub problem_threads: usize,
    pub issue_counts: BTreeMap<String, usize>,
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

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteArchivedRequest {
    pub codex_home: String,
    pub thread_ids: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchiveRequest {
    pub codex_home: String,
    pub thread_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ArchiveResult {
    pub changed_threads: Vec<String>,
    pub backup_dir: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DeleteArchivedResult {
    pub deleted_threads: Vec<String>,
    pub backup_dir: Option<String>,
    pub dry_run: bool,
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

pub fn default_codex_home() -> String {
    default_codex_home_from_env(|name| std::env::var_os(name))
        .display()
        .to_string()
}

fn default_codex_home_from_env(mut read_env: impl FnMut(&str) -> Option<OsString>) -> PathBuf {
    if let Some(codex_home) = non_empty_env(&mut read_env, "CODEX_HOME") {
        return PathBuf::from(codex_home);
    }
    if let Some(user_profile) = non_empty_env(&mut read_env, "USERPROFILE") {
        return PathBuf::from(user_profile).join(".codex");
    }
    if let (Some(home_drive), Some(home_path)) = (
        non_empty_env(&mut read_env, "HOMEDRIVE"),
        non_empty_env(&mut read_env, "HOMEPATH"),
    ) {
        return PathBuf::from(format!(
            "{}{}",
            home_drive.to_string_lossy(),
            home_path.to_string_lossy()
        ))
        .join(".codex");
    }
    if let Some(home) = non_empty_env(&mut read_env, "HOME") {
        return PathBuf::from(home).join(".codex");
    }
    PathBuf::from(".codex")
}

fn non_empty_env(
    read_env: &mut impl FnMut(&str) -> Option<OsString>,
    name: &str,
) -> Option<OsString> {
    read_env(name).filter(|value| !value.is_empty())
}

pub fn preview_provider_migration(request: MigrationRequest) -> Result<MigrationResult> {
    migration::preview_provider_migration(request)
}

pub fn apply_provider_migration(request: MigrationRequest) -> Result<MigrationResult> {
    migration::apply_provider_migration(request)
}

pub fn preview_delete_archived_sessions(
    request: DeleteArchivedRequest,
) -> Result<DeleteArchivedResult> {
    deletion::preview_delete_archived_sessions(request)
}

pub fn apply_delete_archived_sessions(
    request: DeleteArchivedRequest,
) -> Result<DeleteArchivedResult> {
    deletion::apply_delete_archived_sessions(request)
}

pub fn apply_archive_sessions(request: ArchiveRequest) -> Result<ArchiveResult> {
    archive::apply_archive_sessions(request)
}

pub fn apply_activate_sessions(request: ArchiveRequest) -> Result<ArchiveResult> {
    archive::apply_activate_sessions(request)
}

pub fn switch_provider_and_restart(
    request: ProviderRestartRequest,
) -> Result<ProviderRestartResult> {
    restart::switch_provider_and_restart(request)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;
    use std::path::PathBuf;

    #[test]
    fn default_codex_home_uses_windows_user_profile() {
        let path = default_codex_home_from_env(|name| match name {
            "USERPROFILE" => Some(OsString::from(r"C:\Users\jianrui")),
            _ => None,
        });

        assert_eq!(path, PathBuf::from(r"C:\Users\jianrui").join(".codex"));
    }
}

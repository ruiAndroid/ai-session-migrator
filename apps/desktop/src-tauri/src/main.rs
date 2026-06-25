#![cfg_attr(
    all(target_os = "windows", not(debug_assertions)),
    windows_subsystem = "windows"
)]

use ai_session_migrator::codex::{
    self, ArchiveRequest, ArchiveResult, CommandError, DeleteArchivedRequest, DeleteArchivedResult,
    MigrationRequest, MigrationResult, ProviderRestartRequest, ProviderRestartResult, ScanResponse,
    SessionTranscript, SessionTranscriptRequest,
};
use std::process::Command;

#[tauri::command]
fn app_health() -> &'static str {
    "ok"
}

#[tauri::command]
fn default_codex_home() -> String {
    codex::default_codex_home()
}

#[tauri::command]
fn scan_codex_home(codex_home: String) -> std::result::Result<ScanResponse, CommandError> {
    codex::scan_codex_home(codex_home)
}

#[tauri::command]
fn preview_provider_migration(
    request: MigrationRequest,
) -> std::result::Result<MigrationResult, CommandError> {
    codex::preview_provider_migration(request)
}

#[tauri::command]
fn apply_provider_migration(
    request: MigrationRequest,
) -> std::result::Result<MigrationResult, CommandError> {
    codex::apply_provider_migration(request)
}

#[tauri::command]
fn preview_delete_archived_sessions(
    request: DeleteArchivedRequest,
) -> std::result::Result<DeleteArchivedResult, CommandError> {
    codex::preview_delete_archived_sessions(request)
}

#[tauri::command]
fn apply_delete_archived_sessions(
    request: DeleteArchivedRequest,
) -> std::result::Result<DeleteArchivedResult, CommandError> {
    codex::apply_delete_archived_sessions(request)
}

#[tauri::command]
fn apply_archive_sessions(
    request: ArchiveRequest,
) -> std::result::Result<ArchiveResult, CommandError> {
    codex::apply_archive_sessions(request)
}

#[tauri::command]
fn apply_activate_sessions(
    request: ArchiveRequest,
) -> std::result::Result<ArchiveResult, CommandError> {
    codex::apply_activate_sessions(request)
}

#[tauri::command]
async fn read_session_transcript(
    request: SessionTranscriptRequest,
) -> std::result::Result<SessionTranscript, CommandError> {
    tauri::async_runtime::spawn_blocking(move || codex::read_session_transcript(request))
        .await
        .map_err(|error| {
            CommandError::new(
                "transcript_task_failed",
                format!("failed to read session transcript: {error}"),
            )
        })?
}

#[tauri::command]
fn switch_provider_and_restart(
    request: ProviderRestartRequest,
) -> std::result::Result<ProviderRestartResult, CommandError> {
    codex::switch_provider_and_restart(request)
}

#[tauri::command]
fn open_path(path: String) -> std::result::Result<(), String> {
    let trimmed_path = path.trim();
    if trimmed_path.is_empty() {
        return Err("path is required".to_string());
    }

    let mut command = if cfg!(target_os = "windows") {
        let mut command = Command::new("explorer");
        command.arg(trimmed_path);
        command
    } else if cfg!(target_os = "macos") {
        let mut command = Command::new("open");
        command.arg(trimmed_path);
        command
    } else {
        let mut command = Command::new("xdg-open");
        command.arg(trimmed_path);
        command
    };

    command
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("failed to open path: {error}"))
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            app_health,
            default_codex_home,
            scan_codex_home,
            preview_provider_migration,
            apply_provider_migration,
            preview_delete_archived_sessions,
            apply_delete_archived_sessions,
            apply_archive_sessions,
            apply_activate_sessions,
            read_session_transcript,
            switch_provider_and_restart,
            open_path
        ])
        .run(tauri::generate_context!())
        .expect("failed to run AI Session Migrator");
}

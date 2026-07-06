#![cfg_attr(
    all(target_os = "windows", not(debug_assertions)),
    windows_subsystem = "windows"
)]

use ai_session_migrator::codex::{
    self, ArchiveRequest, ArchiveResult, CatalogRepairRequest, CatalogRepairResult,
    CatalogRepairScanResponse, CommandError, DeleteArchivedRequest, DeleteArchivedResult,
    MigrationRequest, MigrationResult, ProviderRestartRequest, ProviderRestartResult, ScanResponse,
    SessionTranscript, SessionTranscriptRequest,
};
use std::process::Command;
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    App, AppHandle, Manager, Runtime, WebviewWindow, WindowEvent,
};

const MAIN_WINDOW_LABEL: &str = "main";
const TRAY_ID: &str = "ai-session-migrator-tray";
const TRAY_MENU_SHOW_ID: &str = "tray_show_main_window";
const TRAY_MENU_HIDE_ID: &str = "tray_hide_to_tray";
const TRAY_MENU_QUIT_ID: &str = "tray_quit_app";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TrayMenuAction {
    ShowMainWindow,
    HideToTray,
    QuitApp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TrayMenuItemSpec {
    id: &'static str,
    label: &'static str,
}

fn tray_menu_item_specs() -> [TrayMenuItemSpec; 3] {
    [
        TrayMenuItemSpec {
            id: TRAY_MENU_SHOW_ID,
            label: "打开主窗口",
        },
        TrayMenuItemSpec {
            id: TRAY_MENU_HIDE_ID,
            label: "隐藏到托盘",
        },
        TrayMenuItemSpec {
            id: TRAY_MENU_QUIT_ID,
            label: "退出应用",
        },
    ]
}

fn tray_menu_action(menu_id: &str) -> Option<TrayMenuAction> {
    match menu_id {
        TRAY_MENU_SHOW_ID => Some(TrayMenuAction::ShowMainWindow),
        TRAY_MENU_HIDE_ID => Some(TrayMenuAction::HideToTray),
        TRAY_MENU_QUIT_ID => Some(TrayMenuAction::QuitApp),
        _ => None,
    }
}

fn should_show_main_window_from_tray_event(event: &TrayIconEvent) -> bool {
    match event {
        TrayIconEvent::Click {
            button,
            button_state,
            ..
        } => should_show_main_window_from_tray_click(*button, *button_state),
        _ => false,
    }
}

fn should_show_main_window_from_tray_click(
    button: MouseButton,
    button_state: MouseButtonState,
) -> bool {
    button == MouseButton::Left && button_state == MouseButtonState::Up
}

fn setup_system_tray(app: &mut App) -> tauri::Result<()> {
    let menu = build_tray_menu(app.handle())?;
    let app_handle = app.handle().clone();
    let mut tray = TrayIconBuilder::with_id(TRAY_ID)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_tray_icon_event(move |_tray, event| {
            if should_show_main_window_from_tray_event(&event) {
                show_main_window(&app_handle);
            }
        })
        .tooltip("AI Session Migrator");

    if let Some(icon) = app.default_window_icon().cloned() {
        tray = tray.icon(icon);
    }

    tray.build(app)?;
    Ok(())
}

fn build_tray_menu<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Menu<R>> {
    let specs = tray_menu_item_specs();
    let show = MenuItem::with_id(app, specs[0].id, specs[0].label, true, None::<&str>)?;
    let hide = MenuItem::with_id(app, specs[1].id, specs[1].label, true, None::<&str>)?;
    let quit = MenuItem::with_id(app, specs[2].id, specs[2].label, true, None::<&str>)?;
    Menu::with_items(app, &[&show, &hide, &quit])
}

fn handle_tray_menu_action<R: Runtime>(app: &AppHandle<R>, action: TrayMenuAction) {
    match action {
        TrayMenuAction::ShowMainWindow => show_main_window(app),
        TrayMenuAction::HideToTray => hide_main_window(app),
        TrayMenuAction::QuitApp => app.exit(0),
    }
}

fn show_main_window<R: Runtime>(app: &AppHandle<R>) {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) {
        show_window(&window);
    }
}

fn show_window<R: Runtime>(window: &WebviewWindow<R>) {
    let _ = window.show();
    let _ = window.unminimize();
    let _ = window.set_focus();
}

fn hide_main_window<R: Runtime>(app: &AppHandle<R>) {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) {
        let _ = window.hide();
    }
}

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
fn scan_codex_catalog_repair(
    codex_home: String,
) -> std::result::Result<CatalogRepairScanResponse, CommandError> {
    codex::scan_codex_catalog_repair(codex_home)
}

#[tauri::command]
fn preview_provider_migration(
    request: MigrationRequest,
) -> std::result::Result<MigrationResult, CommandError> {
    codex::preview_provider_migration(request)
}

#[tauri::command]
fn preview_codex_catalog_repair(
    request: CatalogRepairRequest,
) -> std::result::Result<CatalogRepairResult, CommandError> {
    codex::preview_codex_catalog_repair(request)
}

#[tauri::command]
fn apply_provider_migration(
    request: MigrationRequest,
) -> std::result::Result<MigrationResult, CommandError> {
    codex::apply_provider_migration(request)
}

#[tauri::command]
fn apply_codex_catalog_repair(
    request: CatalogRepairRequest,
) -> std::result::Result<CatalogRepairResult, CommandError> {
    codex::apply_codex_catalog_repair(request)
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
        .setup(|app| {
            setup_system_tray(app)?;
            Ok(())
        })
        .on_window_event(|window, event| {
            if matches!(event, WindowEvent::CloseRequested { .. }) && window.label() == MAIN_WINDOW_LABEL
            {
                if let WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                }
                let _ = window.hide();
            }
        })
        .on_menu_event(|app, event| {
            if let Some(action) = tray_menu_action(event.id().as_ref()) {
                handle_tray_menu_action(app, action);
            }
        })
        .invoke_handler(tauri::generate_handler![
            app_health,
            default_codex_home,
            scan_codex_home,
            scan_codex_catalog_repair,
            preview_provider_migration,
            preview_codex_catalog_repair,
            apply_provider_migration,
            apply_codex_catalog_repair,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tray_menu_specs_expose_expected_user_actions() {
        let specs = tray_menu_item_specs();

        assert_eq!(specs.len(), 3);
        assert_eq!(specs[0].id, "tray_show_main_window");
        assert_eq!(specs[0].label, "打开主窗口");
        assert_eq!(specs[1].id, "tray_hide_to_tray");
        assert_eq!(specs[1].label, "隐藏到托盘");
        assert_eq!(specs[2].id, "tray_quit_app");
        assert_eq!(specs[2].label, "退出应用");
    }

    #[test]
    fn tray_menu_action_maps_known_ids_and_ignores_unknown_ids() {
        assert_eq!(
            tray_menu_action("tray_show_main_window"),
            Some(TrayMenuAction::ShowMainWindow)
        );
        assert_eq!(
            tray_menu_action("tray_hide_to_tray"),
            Some(TrayMenuAction::HideToTray)
        );
        assert_eq!(tray_menu_action("tray_quit_app"), Some(TrayMenuAction::QuitApp));
        assert_eq!(tray_menu_action("anything_else"), None);
    }

    #[test]
    fn tray_left_click_release_shows_main_window() {
        assert!(should_show_main_window_from_tray_click(
            MouseButton::Left,
            MouseButtonState::Up
        ));
        assert!(!should_show_main_window_from_tray_click(
            MouseButton::Left,
            MouseButtonState::Down
        ));
        assert!(!should_show_main_window_from_tray_click(
            MouseButton::Right,
            MouseButtonState::Up
        ));
    }
}

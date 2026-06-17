#[tauri::command]
fn app_health() -> &'static str {
    "ok"
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![app_health])
        .run(tauri::generate_context!())
        .expect("failed to run AI Session Migrator");
}

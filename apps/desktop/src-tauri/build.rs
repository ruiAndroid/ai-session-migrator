fn main() {
    if cfg!(feature = "desktop") {
        tauri_build::build();
    }
}

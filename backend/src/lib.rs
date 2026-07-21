pub mod commands;
pub mod config;
pub mod error;
pub mod processing;

#[cfg(feature = "desktop")]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .invoke_handler(tauri::generate_handler![commands::get_app_version])
        .run(tauri::generate_context!())
        .expect("error while running RnD Data Processing");
}

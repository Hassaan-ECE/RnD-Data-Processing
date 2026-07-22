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
        .invoke_handler(tauri::generate_handler![
            commands::get_app_version,
            commands::scan_data_folder,
            commands::load_setup_file,
            commands::run_system_208v_report,
            commands::preview_load_bands_async,
            commands::open_path
        ])
        .run(tauri::generate_context!())
        .expect("error while running RnD Data Processing");
}

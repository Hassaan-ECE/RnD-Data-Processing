use crate::config::load_embedded_config;
use crate::error::{AppError, AppResult};
use crate::processing::discover::{discover_data_folder, DiscoveryResult};
use crate::processing::setup::{load_setup_targets, SetupLoadResult};

#[cfg(feature = "desktop")]
use crate::processing::pipeline::{self, PipelineInput, PipelineResult};
use crate::processing::preview::{self, BandPreviewResult, PreviewInput};

#[cfg_attr(feature = "desktop", tauri::command)]
pub fn get_app_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg_attr(feature = "desktop", tauri::command)]
pub fn scan_data_folder(data_folder: String, test_id: String) -> AppResult<DiscoveryResult> {
    let config = load_embedded_config()?;
    let test = config
        .test(&test_id)
        .ok_or_else(|| AppError::Message(format!("Unknown test id '{test_id}'")))?;
    discover_data_folder(data_folder, test)
}

#[cfg_attr(feature = "desktop", tauri::command)]
pub fn load_setup_file(setup_path: String, test_id: String) -> AppResult<SetupLoadResult> {
    let config = load_embedded_config()?;
    let test = config
        .test(&test_id)
        .ok_or_else(|| AppError::Message(format!("Unknown test id '{test_id}'")))?;
    load_setup_targets(setup_path, test)
}

#[cfg(feature = "desktop")]
#[tauri::command]
pub async fn run_system_208v_report(input: PipelineInput) -> AppResult<PipelineResult> {
    tauri::async_runtime::spawn_blocking(move || pipeline::run_system_208v(input))
        .await
        .map_err(|error| AppError::Message(format!("Report worker failed: {error}")))?
}

#[cfg(feature = "desktop")]
#[tauri::command]
pub async fn run_report(test_id: String, input: PipelineInput) -> AppResult<PipelineResult> {
    tauri::async_runtime::spawn_blocking(move || pipeline::run_test(&test_id, input))
        .await
        .map_err(|error| AppError::Message(format!("Report worker failed: {error}")))?
}

#[cfg_attr(feature = "desktop", tauri::command)]
pub fn preview_load_bands(input: PreviewInput) -> AppResult<BandPreviewResult> {
    preview::preview_load_bands(input)
}

// Async wrapper keeps heavy CSV work off the UI thread in desktop builds.
#[cfg(feature = "desktop")]
#[tauri::command]
pub async fn preview_load_bands_async(input: PreviewInput) -> AppResult<BandPreviewResult> {
    tauri::async_runtime::spawn_blocking(move || preview::preview_load_bands(input))
        .await
        .map_err(|error| AppError::Message(format!("Preview worker failed: {error}")))?
}

#[cfg(feature = "desktop")]
#[tauri::command]
pub fn open_path(app: tauri::AppHandle, path: String) -> AppResult<()> {
    use tauri_plugin_opener::OpenerExt;

    let target = std::path::PathBuf::from(&path);
    if !target.exists() {
        return Err(AppError::Message(format!(
            "Path does not exist: {}",
            target.display()
        )));
    }
    app.opener()
        .open_path(path, None::<String>)
        .map_err(|error| AppError::Message(error.to_string()))
}

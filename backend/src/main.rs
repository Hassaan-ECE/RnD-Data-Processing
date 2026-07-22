// Hide the extra console window on Windows release builds (GUI app only).
// Debug / `tauri dev` still attaches a console for logs.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    rnd_data_processing_lib::run();
}

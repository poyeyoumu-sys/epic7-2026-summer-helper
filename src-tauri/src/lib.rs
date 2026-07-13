mod commands;
mod config;
mod controller;
mod events;
mod game;
mod recognition;
mod state;

use std::path::PathBuf;
use tauri::Manager;

use config::{load_settings, AppPaths};
use state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let mut resource_dir = app.path().resource_dir()?;
            if !resource_dir.join("resources/config/recognition_config.json").exists() {
                resource_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            }
            let data_dir = app.path().app_data_dir()?;
            let paths = AppPaths::new(resource_dir, data_dir)?;
            let settings = load_settings(&paths.settings_file).unwrap_or_default();
            app.manage(AppState::new(paths, settings));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::load_settings,
            commands::save_settings,
            commands::get_status,
            commands::discover_devices,
            commands::connect_device,
            commands::save_screenshot,
            commands::start_runner,
            commands::stop_runner,
        ])
        .run(tauri::generate_context!())
        .expect("运行 Tauri 应用失败");
}

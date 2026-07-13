use std::{collections::HashMap, fs, sync::atomic::Ordering};
use tauri::{AppHandle, State};

use crate::{
    config::{self, AppSettings},
    controller::{create_controller, discover_adb_devices, discover_maa_devices, DeviceInfo},
    events::{emit_log, emit_status, RuntimeStatus},
    game::{RunnerEngine, RunnerMode},
    recognition::StateReader,
    state::AppState,
};

#[tauri::command]
pub fn load_settings(state: State<'_, AppState>) -> AppSettings {
    state.settings.lock().clone()
}

#[tauri::command]
pub fn save_settings(settings: AppSettings, state: State<'_, AppState>) -> Result<(), String> {
    config::save_settings(&state.paths.settings_file, &settings).map_err(|e| e.to_string())?;
    *state.settings.lock() = settings.clone();
    state.status.lock().strategy = settings.strategy_mode;
    Ok(())
}

#[tauri::command]
pub fn get_status(state: State<'_, AppState>) -> RuntimeStatus {
    state.status.lock().clone()
}

#[tauri::command]
pub fn discover_devices(state: State<'_, AppState>, app: AppHandle) -> Result<Vec<DeviceInfo>, String> {
    let mut map = HashMap::<String, DeviceInfo>::new();
    match discover_maa_devices(&state.paths) {
        Ok(devices) => for device in devices { map.insert(device.serial.clone(), device); },
        Err(error) => emit_log(&app, "WARN", "maa", format!("MAA 设备发现不可用：{}", error)),
    }
    match discover_adb_devices(&state.paths.adb_path) {
        Ok(devices) => for device in devices { map.entry(device.serial.clone()).or_insert(device); },
        Err(error) => emit_log(&app, "WARN", "adb", format!("ADB 设备发现失败：{}", error)),
    }
    let mut devices = map.into_values().collect::<Vec<_>>();
    devices.sort_by(|a,b| a.serial.cmp(&b.serial));
    Ok(devices)
}

#[tauri::command]
pub fn connect_device(settings: AppSettings, state: State<'_, AppState>, app: AppHandle) -> Result<RuntimeStatus, String> {
    if state.running.load(Ordering::SeqCst) { return Err("流程运行中，不能重新连接设备".into()); }
    config::save_settings(&state.paths.settings_file, &settings).map_err(|e| e.to_string())?;
    let controller = create_controller(&settings, &state.paths).map_err(|e| e.to_string())?;
    let mut status = state.status.lock();
    status.connected = true;
    status.device = controller.serial().to_string();
    status.backend = controller.backend_name().to_string();
    status.phase = "已连接".into();
    status.strategy = settings.strategy_mode.clone();
    *state.controller.lock() = Some(controller);
    *state.settings.lock() = settings;
    emit_status(&app, &status);
    emit_log(&app, "INFO", "device", format!("连接成功：{} · {}", status.device, status.backend));
    Ok(status.clone())
}

#[tauri::command]
pub fn save_screenshot(state: State<'_, AppState>, app: AppHandle) -> Result<String, String> {
    let controller = state.controller.lock().clone().ok_or_else(|| "请先连接设备".to_string())?;
    let image = controller.capture().map_err(|e| e.to_string())?;
    let name = format!("screen_{}.png", chrono::Local::now().format("%Y%m%d_%H%M%S"));
    let path = state.paths.screenshots_dir.join(name);
    image.save(&path).map_err(|e| e.to_string())?;
    emit_log(&app, "INFO", "screenshot", format!("截图已保存：{}", path.display()));
    Ok(path.to_string_lossy().to_string())
}

#[tauri::command]
pub fn start_runner(mode: RunnerMode, settings: AppSettings, state: State<'_, AppState>, app: AppHandle) -> Result<(), String> {
    if state.running.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst).is_err() {
        return Err("已有流程正在运行".into());
    }
    config::save_settings(&state.paths.settings_file, &settings).map_err(|e| e.to_string())?;
    *state.settings.lock() = settings.clone();
    state.reset_stop();

    let controller = match state.controller.lock().clone() {
        Some(controller) if controller.serial() == settings.serial || settings.serial.eq_ignore_ascii_case("auto") => controller,
        _ => create_controller(&settings, &state.paths).map_err(|e| {
            state.running.store(false, Ordering::SeqCst);
            e.to_string()
        })?,
    };
    *state.controller.lock() = Some(controller.clone());
    let reader = StateReader::load(&state.paths.recognition_config, &state.paths.template_dir).map_err(|e| {
        state.running.store(false, Ordering::SeqCst);
        e.to_string()
    })?;

    {
        let mut status = state.status.lock();
        status.running = true;
        status.connected = true;
        status.device = controller.serial().to_string();
        status.backend = controller.backend_name().to_string();
        status.phase = "正在启动".into();
        status.strategy = settings.strategy_mode.clone();
        emit_status(&app, &status);
    }

    let stop = state.stop_flag.clone();
    let running = state.running.clone();
    let status = state.status.clone();
    let thread_app = app.clone();
    std::thread::spawn(move || {
        let engine = RunnerEngine::new(thread_app.clone(), controller, reader, settings, stop, status.clone());
        if let Err(error) = engine.run(mode) {
            emit_log(&thread_app, "ERROR", "runner", error.to_string());
        }
        running.store(false, Ordering::SeqCst);
        let mut current = status.lock();
        current.running = false;
        current.phase = "流程已结束".into();
        emit_status(&thread_app, &current);
        emit_log(&thread_app, "INFO", "runner", "脚本已结束");
    });
    Ok(())
}

#[tauri::command]
pub fn stop_runner(state: State<'_, AppState>, app: AppHandle) {
    state.request_stop();
    emit_log(&app, "WARN", "runner", "已发送停止请求");
}

mod adb;
mod maa;

use anyhow::{bail, Result};
use image::DynamicImage;
use serde::{Deserialize, Serialize};
use std::{path::Path, sync::Arc};

use crate::config::{AppPaths, AppSettings, CaptureBackendKind};

pub use adb::{discover_adb_devices, AdbController};
pub use maa::{discover_maa_devices, MaaController};

pub trait DeviceController: Send + Sync {
    fn backend_name(&self) -> &'static str;
    fn serial(&self) -> &str;
    fn capture(&self) -> Result<DynamicImage>;
    fn tap(&self, x: i32, y: i32) -> Result<()>;
}

pub type SharedController = Arc<dyn DeviceController>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub serial: String,
    pub name: String,
    pub source: String,
    pub supports_emulator_extras: bool,
}

pub fn resolve_serial(requested: &str, devices: &[DeviceInfo]) -> Result<String> {
    let requested = requested.trim();
    if requested.is_empty() || requested.eq_ignore_ascii_case("auto") {
        return devices.first().map(|d| d.serial.clone()).ok_or_else(|| anyhow::anyhow!("没有检测到可用设备"));
    }
    Ok(requested.to_string())
}

pub fn create_controller(settings: &AppSettings, paths: &AppPaths) -> Result<SharedController> {
    match settings.capture_backend {
        CaptureBackendKind::MaaEmulatorExtras => {
            match MaaController::connect(settings, paths) {
                Ok(controller) => Ok(Arc::new(controller)),
                Err(error) if settings.fallback_to_adb => {
                    let devices = discover_adb_devices(&paths.adb_path).unwrap_or_default();
                    let serial = resolve_serial(&settings.serial, &devices)?;
                    Ok(Arc::new(AdbController::new(paths.adb_path.clone(), serial)?))
                }
                Err(error) => Err(error),
            }
        }
        CaptureBackendKind::AdbScreencap => {
            let devices = discover_adb_devices(&paths.adb_path).unwrap_or_default();
            let serial = resolve_serial(&settings.serial, &devices)?;
            Ok(Arc::new(AdbController::new(paths.adb_path.clone(), serial)?))
        }
    }
}

pub fn ensure_file(path: &Path, label: &str) -> Result<()> {
    if !path.exists() { bail!("{}不存在：{}", label, path.display()); }
    Ok(())
}

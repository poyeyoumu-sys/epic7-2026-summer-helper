use anyhow::{bail, Context, Result};
use image::{DynamicImage, ImageBuffer, Rgb};
use maa_framework::{controller::{AdbControllerBuilder, Controller}, sys, toolkit::Toolkit};
use std::{env, path::{Path, PathBuf}, sync::OnceLock};
use walkdir::WalkDir;

use crate::config::{AppPaths, AppSettings};
use super::{DeviceController, DeviceInfo, resolve_serial};

static MAA_LOADED: OnceLock<()> = OnceLock::new();

pub struct MaaController {
    controller: Controller,
    serial: String,
}

impl MaaController {
    pub fn connect(settings: &AppSettings, paths: &AppPaths) -> Result<Self> {
        load_runtime(&paths.maa_runtime_dir, &paths.data_dir)?;
        let adb = paths.adb_path.to_string_lossy().to_string();
        let devices = Toolkit::find_adb_devices_with_adb(&adb).context("MAA Toolkit 搜索设备失败")?;
        let infos: Vec<DeviceInfo> = devices.iter().map(|d| DeviceInfo {
            serial: d.address.clone(),
            name: d.name.clone(),
            source: "maa".into(),
            supports_emulator_extras: d.screencap_methods & (sys::MaaAdbScreencapMethod_EmulatorExtras as u64) != 0,
        }).collect();
        let serial = resolve_serial(&settings.serial, &infos)?;
        let device = devices.iter().find(|d| d.address == serial).ok_or_else(|| anyhow::anyhow!("MAA 未找到指定设备：{}", serial))?;
        let extras = sys::MaaAdbScreencapMethod_EmulatorExtras as sys::MaaAdbScreencapMethod;
        if device.screencap_methods & extras == 0 {
            bail!("当前设备不支持 MAA EmulatorExtras：{}", serial);
        }
        let config = serde_json::to_string(&device.config)?;
        let agent_path = find_agent_path(&paths.maa_runtime_dir).unwrap_or_else(|| paths.maa_runtime_dir.clone());
        let controller = AdbControllerBuilder::new(&adb, &serial)
            .screencap_methods(extras)
            .input_methods(device.input_methods as sys::MaaAdbInputMethod)
            .config(&config)
            .agent_path(&agent_path.to_string_lossy())
            .build()
            .context("创建 MAA EmulatorExtras Controller 失败")?;
        let id = controller.post_connection()?;
        controller.wait(id);
        if !controller.connected() { bail!("MAA Controller 连接失败：{}", serial); }
        controller.set_screenshot_use_raw_size(true)?;
        Ok(Self { controller, serial })
    }
}

impl DeviceController for MaaController {
    fn backend_name(&self) -> &'static str { "MAA EmulatorExtras" }
    fn serial(&self) -> &str { &self.serial }

    fn capture(&self) -> Result<DynamicImage> {
        let id = self.controller.post_screencap()?;
        self.controller.wait(id);
        let buffer = self.controller.cached_image()?;
        let width = buffer.width() as u32;
        let height = buffer.height() as u32;
        let raw = buffer.raw_data().ok_or_else(|| anyhow::anyhow!("MAA 截图缓冲区为空"))?;
        let expected = width as usize * height as usize * 3;
        if raw.len() < expected { bail!("MAA BGR 缓冲长度异常：{} < {}", raw.len(), expected); }
        let mut rgb = Vec::with_capacity(expected);
        for bgr in raw[..expected].chunks_exact(3) {
            rgb.extend_from_slice(&[bgr[2], bgr[1], bgr[0]]);
        }
        let image: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::from_raw(width, height, rgb)
            .ok_or_else(|| anyhow::anyhow!("构造 MAA RGB 图像失败"))?;
        Ok(DynamicImage::ImageRgb8(image))
    }

    fn tap(&self, x: i32, y: i32) -> Result<()> {
        let id = self.controller.post_click(x, y)?;
        self.controller.wait(id);
        Ok(())
    }
}

pub fn discover_maa_devices(paths: &AppPaths) -> Result<Vec<DeviceInfo>> {
    load_runtime(&paths.maa_runtime_dir, &paths.data_dir)?;
    let adb = paths.adb_path.to_string_lossy().to_string();
    Ok(Toolkit::find_adb_devices_with_adb(&adb)?.into_iter().map(|d| DeviceInfo {
        supports_emulator_extras: d.screencap_methods & (sys::MaaAdbScreencapMethod_EmulatorExtras as u64) != 0,
        serial: d.address,
        name: d.name,
        source: "maa".into(),
    }).collect())
}

fn load_runtime(runtime_dir: &Path, data_dir: &Path) -> Result<()> {
    if MAA_LOADED.get().is_some() { return Ok(()); }
    let dll = find_file(runtime_dir, "MaaFramework.dll").ok_or_else(|| anyhow::anyhow!(
        "缺少 MaaFramework.dll。请先运行 scripts\\download-maa-runtime.ps1"
    ))?;
    if let Some(parent) = dll.parent() {
        let old_path = env::var_os("PATH").unwrap_or_default();
        let mut paths = vec![parent.to_path_buf()];
        paths.extend(env::split_paths(&old_path));
        env::set_var("PATH", env::join_paths(paths)?);
        #[cfg(windows)]
        unsafe {
            use std::os::windows::ffi::OsStrExt;
            let wide: Vec<u16> = parent.as_os_str().encode_wide().chain(Some(0)).collect();
            windows_sys::Win32::System::LibraryLoader::SetDllDirectoryW(wide.as_ptr());
        }
    }
    maa_framework::load_library(&dll).map_err(|error| {
        anyhow::anyhow!(
            "加载 MaaFramework.dll 失败，路径：{}，错误：{}",
            dll.display(),
            error
        )
    })?;
    let maa_user = data_dir.join("maa");
    std::fs::create_dir_all(&maa_user)?;
    Toolkit::init_option(&maa_user.to_string_lossy(), "{}")?;
    let _ = MAA_LOADED.set(());
    Ok(())
}

fn find_file(root: &Path, name: &str) -> Option<PathBuf> {
    WalkDir::new(root).into_iter().filter_map(Result::ok)
        .find(|e| e.file_type().is_file() && e.file_name().to_string_lossy().eq_ignore_ascii_case(name))
        .map(|e| e.path().to_path_buf())
}

fn find_agent_path(root: &Path) -> Option<PathBuf> {
    for name in ["MaaAgentBinary.exe", "MaaAgentBinary"] {
        if let Some(path) = find_file(root, name) { return path.parent().map(Path::to_path_buf); }
    }
    None
}
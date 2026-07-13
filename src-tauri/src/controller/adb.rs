use anyhow::{bail, Context, Result};
use image::DynamicImage;
use std::{path::PathBuf, process::{Command, Stdio}};

use super::{DeviceController, DeviceInfo};

pub struct AdbController {
    adb_path: PathBuf,
    serial: String,
}

impl AdbController {
    pub fn new(adb_path: PathBuf, serial: String) -> Result<Self> {
        if !adb_path.exists() { bail!("ADB 不存在：{}", adb_path.display()); }
        let instance = Self { adb_path, serial };
        instance.ensure_connected()?;
        Ok(instance)
    }

    fn command(&self) -> Command {
        let mut cmd = Command::new(&self.adb_path);
        cmd.arg("-s").arg(&self.serial);
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x08000000);
        }
        cmd
    }

    fn ensure_connected(&self) -> Result<()> {
        if self.serial.contains(':') {
            let mut connect = Command::new(&self.adb_path);
            connect.arg("connect").arg(&self.serial);
            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                connect.creation_flags(0x08000000);
            }
            let _ = connect.output();
        }
        let output = self.command().arg("get-state").output().context("执行 adb get-state 失败")?;
        if !output.status.success() { bail!("设备未连接：{}", self.serial); }
        Ok(())
    }
}

impl DeviceController for AdbController {
    fn backend_name(&self) -> &'static str { "ADB screencap" }
    fn serial(&self) -> &str { &self.serial }

    fn capture(&self) -> Result<DynamicImage> {
        let output = self.command()
            .args(["exec-out", "screencap", "-p"])
            .stdout(Stdio::piped())
            .output()
            .context("执行 adb screencap 失败")?;
        if !output.status.success() { bail!("ADB 截图失败：{}", String::from_utf8_lossy(&output.stderr)); }
        image::load_from_memory(&output.stdout).context("解析 ADB PNG 截图失败")
    }

    fn tap(&self, x: i32, y: i32) -> Result<()> {
        let output = self.command().args(["shell", "input", "tap", &x.to_string(), &y.to_string()]).output()?;
        if !output.status.success() { bail!("ADB 点击失败：{}", String::from_utf8_lossy(&output.stderr)); }
        Ok(())
    }
}

pub fn discover_adb_devices(adb_path: &PathBuf) -> Result<Vec<DeviceInfo>> {
    if !adb_path.exists() { bail!("ADB 不存在：{}", adb_path.display()); }
    let common = ["127.0.0.1:16384", "127.0.0.1:7555", "127.0.0.1:5555", "127.0.0.1:62001", "127.0.0.1:59865", "127.0.0.1:21503"];
    for serial in common {
        let mut command = Command::new(adb_path);
        command.args(["connect", serial]);
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x08000000);
        }
        let _ = command.output();
    }
    let mut command = Command::new(adb_path);
    command.arg("devices");
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    let output = command.output().context("执行 adb devices 失败")?;
    let text = String::from_utf8_lossy(&output.stdout);
    let result = text.lines().filter_map(|line| {
        let mut parts = line.split_whitespace();
        let serial = parts.next()?;
        let state = parts.next()?;
        (state == "device").then(|| DeviceInfo {
            serial: serial.to_string(),
            name: serial.to_string(),
            source: "adb".into(),
            supports_emulator_extras: false,
        })
    }).collect();
    Ok(result)
}

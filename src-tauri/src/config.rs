use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::{fs, path::{Path, PathBuf}};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CaptureBackendKind {
    MaaEmulatorExtras,
    AdbScreencap,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum StrategyMode {
    EquipmentScore,
    Reward32Fixed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManualStateConfig {
    pub pos: i32,
    pub shield: i32,
    pub boost: i32,
    pub lucky: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunnerOptions {
    pub action_step_timeout_ms: u64,
    pub followup_step_timeout_ms: u64,
    pub fast_poll_interval_ms: u64,
    pub skill_select_delay_ms: u64,
    pub skill_select_retry_delay_ms: u64,
    pub skill_select_retry_limit: usize,
    pub skill_cancel_threshold: f32,
    pub skill_soft_confirm_threshold: f32,
    pub unavailable_score_threshold: f32,
    pub no_change_limit: usize,
    pub post200_cutdown_threshold: f32,
    pub post200_cutdown_confirm_hits: usize,
    pub post200_cutdown_confirm_interval_ms: u64,
    pub post200_cutdown_dismiss_timeout_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub serial: String,
    pub capture_backend: CaptureBackendKind,
    pub fallback_to_adb: bool,
    pub strategy_mode: StrategyMode,
    pub manual_state: ManualStateConfig,
    pub runner: RunnerOptions,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            serial: "auto".into(),
            capture_backend: CaptureBackendKind::MaaEmulatorExtras,
            fallback_to_adb: true,
            strategy_mode: StrategyMode::EquipmentScore,
            manual_state: ManualStateConfig { pos: 0, shield: 2, boost: 1, lucky: 2 },
            runner: RunnerOptions {
                action_step_timeout_ms: 1_000,
                followup_step_timeout_ms: 1_000,
                fast_poll_interval_ms: 100,
                skill_select_delay_ms: 200,
                skill_select_retry_delay_ms: 150,
                skill_select_retry_limit: 3,
                skill_cancel_threshold: 0.40,
                skill_soft_confirm_threshold: 0.30,
                unavailable_score_threshold: 0.10,
                no_change_limit: 5,
                post200_cutdown_threshold: 0.95,
                post200_cutdown_confirm_hits: 2,
                post200_cutdown_confirm_interval_ms: 100,
                post200_cutdown_dismiss_timeout_ms: 1_200,
            },
        }
    }
}

#[derive(Debug, Clone)]
pub struct AppPaths {
    pub resource_dir: PathBuf,
    pub data_dir: PathBuf,
    pub settings_file: PathBuf,
    pub recognition_config: PathBuf,
    pub template_dir: PathBuf,
    pub adb_path: PathBuf,
    pub maa_runtime_dir: PathBuf,
    pub screenshots_dir: PathBuf,
}

impl AppPaths {
    pub fn new(resource_dir: PathBuf, data_dir: PathBuf) -> Result<Self> {
        fs::create_dir_all(&data_dir).context("创建应用数据目录失败")?;
        let screenshots_dir = data_dir.join("screenshots");
        fs::create_dir_all(&screenshots_dir).context("创建截图目录失败")?;
        Ok(Self {
            settings_file: data_dir.join("lucky_sprint_settings.json"),
            recognition_config: resource_dir.join("resources/config/recognition_config.json"),
            template_dir: resource_dir.join("resources/templates/Lucky Sprint"),
            adb_path: resource_dir.join("resources/tools/adbutils/binaries/adb.exe"),
            maa_runtime_dir: resource_dir.join("runtime/maa"),
            resource_dir,
            data_dir,
            screenshots_dir,
        })
    }
}

pub fn load_settings(path: &Path) -> Result<AppSettings> {
    if !path.exists() {
        return Ok(AppSettings::default());
    }
    let text = fs::read_to_string(path).context("读取设置文件失败")?;
    serde_json::from_str(&text).context("解析设置文件失败")
}

pub fn save_settings(path: &Path, settings: &AppSettings) -> Result<()> {
    if let Some(parent) = path.parent() { fs::create_dir_all(parent)?; }
    let text = serde_json::to_string_pretty(settings)?;
    fs::write(path, text).context("保存设置文件失败")
}

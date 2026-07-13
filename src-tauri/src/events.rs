use chrono::Local;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};

use crate::config::StrategyMode;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: String,
    pub scope: String,
    pub message: String,
}

pub fn emit_log(app: &AppHandle, level: &str, scope: &str, message: impl Into<String>) {
    let entry = LogEntry {
        timestamp: Local::now().format("%H:%M:%S%.3f").to_string(),
        level: level.to_uppercase(),
        scope: scope.to_string(),
        message: message.into(),
    };
    let _ = app.emit("runner-log", entry);
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeStatus {
    pub running: bool,
    pub connected: bool,
    pub device: String,
    pub backend: String,
    pub phase: String,
    pub pos: Option<i32>,
    pub shield: Option<i32>,
    pub boost: Option<i32>,
    pub lucky: Option<i32>,
    pub lucky_level: Option<i32>,
    pub strategy: StrategyMode,
}

impl Default for RuntimeStatus {
    fn default() -> Self {
        Self {
            running: false,
            connected: false,
            device: String::new(),
            backend: String::new(),
            phase: "就绪".into(),
            pos: None,
            shield: None,
            boost: None,
            lucky: None,
            lucky_level: None,
            strategy: StrategyMode::EquipmentScore,
        }
    }
}

pub fn emit_status(app: &AppHandle, status: &RuntimeStatus) {
    let _ = app.emit("runner-status", status.clone());
}

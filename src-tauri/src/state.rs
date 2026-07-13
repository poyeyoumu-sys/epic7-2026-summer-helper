use parking_lot::Mutex;
use std::sync::{atomic::{AtomicBool, Ordering}, Arc};

use crate::{config::{AppPaths, AppSettings}, controller::SharedController, events::RuntimeStatus};

pub struct AppState {
    pub paths: AppPaths,
    pub settings: Mutex<AppSettings>,
    pub controller: Mutex<Option<SharedController>>,
    pub status: Arc<Mutex<RuntimeStatus>>,
    pub stop_flag: Arc<AtomicBool>,
    pub running: Arc<AtomicBool>,
}

impl AppState {
    pub fn new(paths: AppPaths, settings: AppSettings) -> Self {
        let mut status = RuntimeStatus::default();
        status.strategy = settings.strategy_mode.clone();
        Self {
            paths,
            settings: Mutex::new(settings),
            controller: Mutex::new(None),
            status: Arc::new(Mutex::new(status)),
            stop_flag: Arc::new(AtomicBool::new(false)),
            running: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn request_stop(&self) { self.stop_flag.store(true, Ordering::SeqCst); }
    pub fn reset_stop(&self) { self.stop_flag.store(false, Ordering::SeqCst); }
}

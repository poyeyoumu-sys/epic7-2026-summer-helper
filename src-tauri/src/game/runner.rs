use anyhow::{bail, Result};
use image::DynamicImage;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::{sync::{atomic::{AtomicBool, Ordering}, Arc}, thread, time::{Duration, Instant}};
use tauri::AppHandle;

use crate::{
    config::AppSettings,
    controller::SharedController,
    events::{emit_log, emit_status, RuntimeStatus},
    recognition::StateReader,
};

use super::{
    core::{apply_progress, TARGET_POS},
    model::{lucky_config, Action, GameState},
    strategy::StrategyRuntime,
};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunnerMode { RecognitionTest, StartZero, TakeoverManual }

#[derive(Debug, Clone)]
struct PendingAction {
    action: Action,
    before: GameState,
    started: Instant,
}

enum TapOutcome { Confirmed, Retry, Unavailable }

pub struct RunnerEngine {
    app: AppHandle,
    controller: SharedController,
    reader: StateReader,
    settings: AppSettings,
    stop: Arc<AtomicBool>,
    status: Arc<Mutex<RuntimeStatus>>,
}

impl RunnerEngine {
    pub fn new(
        app: AppHandle,
        controller: SharedController,
        reader: StateReader,
        settings: AppSettings,
        stop: Arc<AtomicBool>,
        status: Arc<Mutex<RuntimeStatus>>,
    ) -> Self {
        Self { app, controller, reader, settings, stop, status }
    }

    pub fn run(mut self, mode: RunnerMode) -> Result<()> {
        self.set_phase("启动流程");
        emit_log(&self.app, "INFO", "runner", format!("使用截图后端：{}", self.controller.backend_name()));
        match mode {
            RunnerMode::RecognitionTest => self.recognition_test(),
            RunnerMode::StartZero => self.run_from_zero(),
            RunnerMode::TakeoverManual => self.run_takeover(),
        }
    }

    fn recognition_test(&mut self) -> Result<()> {
        self.set_phase("识别测试");
        let image = self.controller.capture()?;
        let level = self.reader.read_lucky_level(&image);
        let shield = self.reader.read_shield(&image);
        let meter = self.reader.read_meter(&image);
        let page = self.reader.read_page(&image);
        let cut = self.reader.read_cutdown(&image, self.settings.runner.post200_cutdown_threshold);
        emit_log(&self.app, if self.reader.main_page_ok(&image) { "INFO" } else { "WARN" }, "recognition", format!("主页面按钮：{}", if self.reader.main_page_ok(&image) { "识别成功" } else { "识别不完整" }));
        emit_log(&self.app, "INFO", "recognition", format!("幸运等级：{:?}，best={} score={:.2}", level.value, level.name, level.score));
        emit_log(&self.app, "INFO", "recognition", format!("保护数量：{:?}，best={} score={:.2}", shield.value, shield.name, shield.score));
        emit_log(&self.app, "INFO", "recognition", format!("当前米数：{:?}，best={} score={:.2}", meter.value, meter.name, meter.score));
        emit_log(&self.app, "INFO", "recognition", format!("页面={} score={:.2}，cutdown={} score={:.2}", page.0, page.1, cut.0, cut.1));
        Ok(())
    }

    fn run_from_zero(&mut self) -> Result<()> {
        self.prepare_main_page()?;
        let mut round = 1usize;
        while !self.stopped() {
            emit_log(&self.app, "INFO", "round", format!("开始第 {} 轮", round));
            if !self.run_normal_round()? { break; }
            round += 1;
        }
        Ok(())
    }

    fn run_takeover(&mut self) -> Result<()> {
        let image = self.prepare_main_page()?;
        let level = self.read_level_or_default(&image);
        let manual = &self.settings.manual_state;
        let state = GameState::manual(manual.pos, manual.shield, manual.boost, manual.lucky, level);
        emit_log(&self.app, "INFO", "runner", format!("中途接管：LV{}，{}m，保护={}，助跑={}，幸运={}", level, state.pos, state.shield, state.boost, state.lucky));
        if self.run_round(state, level)? {
            let mut round = 2usize;
            while !self.stopped() {
                emit_log(&self.app, "INFO", "round", format!("开始第 {} 轮", round));
                if !self.run_normal_round()? { break; }
                round += 1;
            }
        }
        Ok(())
    }

    fn run_normal_round(&mut self) -> Result<bool> {
        let first_image = self.controller.capture()?;
        let image = match self.handle_reward_if_present(first_image)? {
            Some(image) => image,
            None => self.controller.capture()?,
        };
        let level = self.read_level_or_default(&image);
        let shield = self.reader.read_shield(&image);
        let Some(shield_count) = shield.value else {
            emit_log(&self.app, "ERROR", "recognition", format!("保护数量识别失败：best={} score={:.2}", shield.name, shield.score));
            return Ok(false);
        };
        let lucky = lucky_config(level);
        let state = GameState {
            pos: 0,
            shield: shield_count,
            boost: 1,
            lucky: lucky.start,
            got100: false,
            got200: false,
            got_lucky_refill: false,
        };
        emit_log(&self.app, "INFO", "strategy", format!("本轮策略={:?}，LV{}：开局幸运={}，上限={}，补充={}m", self.settings.strategy_mode, level, lucky.start, lucky.max, lucky.refill_meter));
        self.run_round(state, level)
    }

    fn run_round(&mut self, mut state: GameState, lucky_level: i32) -> Result<bool> {
        let mut strategy = StrategyRuntime::new(self.settings.strategy_mode.clone(), lucky_level, state.pos);
        let mut pending: Option<PendingAction> = None;
        let mut no_change = 0usize;

        loop {
            if self.stopped() { return Ok(false); }
            self.update_game_status(&state, lucky_level, "200m前决策");
            let mut image = self.controller.capture()?;

            let (page, score) = self.reader.read_page(&image);
            if page == "get_reward_page" {
                emit_log(&self.app, "INFO", "page", format!("识别到奖励页 score={:.2}，关闭后继续结算", score));
                if let Some(closed) = self.close_reward_page(&image)? { image = closed; } else { continue; }
            } else if page == "howto_get_drink_page" {
                emit_log(&self.app, "STOP", "page", "检测到饮料补充说明页，停止脚本");
                return Ok(false);
            }

            if self.confirm_cutdown(&image, 0.90, 2)? {
                emit_log(&self.app, "INFO", "round", "识别到归零按钮，本轮结束");
                return self.dismiss_cutdown(&image, 1_500);
            }

            let meter = self.reader.read_meter(&image);
            let Some(current_pos) = meter.value else {
                no_change += 1;
                emit_log(&self.app, "WARN", "meter", format!("米数识别失败：best={} score={:.2}，重试 {}/{}", meter.name, meter.score, no_change, self.settings.runner.no_change_limit));
                self.sleep_ms(250);
                continue;
            };
            let shield_read = self.reader.read_shield(&image).value;

            if let Some(p) = pending.clone() {
                if p.before.pos > 0 && current_pos == 0 {
                    emit_log(&self.app, "WARN", "pending", format!("{} 后识别到 0m，本轮结束", p.action.display()));
                    return Ok(true);
                }
                if p.action == Action::Shield && current_pos == p.before.pos && shield_read == Some((p.before.shield - 1).max(0)) {
                    state = p.before;
                    state.shield = (state.shield - 1).max(0);
                    if let Some(message) = strategy.on_shield_failure(p.before.pos, state.shield) { emit_log(&self.app, "WARN", "strategy", message); }
                    emit_log(&self.app, "WARN", "pending", format!("防护失败：{}m，保护 {} -> {}", state.pos, p.before.shield, state.shield));
                    pending = None;
                    continue;
                }
                if current_pos > p.before.pos {
                    state = self.settle_success(p.before, p.action, current_pos, lucky_level);
                    if let Some(message) = strategy.on_success(p.action, p.before.pos, current_pos) { emit_log(&self.app, "INFO", "strategy", message); }
                    emit_log(&self.app, "INFO", "pending", format!("动作结算：{}，{}m -> {}m；保护={}，助跑={}，幸运={}", p.action.display(), p.before.pos, current_pos, state.shield, state.boost, state.lucky));
                    pending = None;
                    no_change = 0;
                    continue;
                }
                if p.action == Action::Boost && current_pos == (p.before.pos - 10).max(0) {
                    emit_log(&self.app, "INFO", "pending", format!("助跑后退动画：{}m -> {}m，继续等待最终结算", p.before.pos, current_pos));
                    self.sleep_ms(200);
                    continue;
                }
                if current_pos == p.before.pos && p.started.elapsed() < Duration::from_secs(12) {
                    emit_log(&self.app, "INFO", "pending", format!("{}仍在等待结算，已等待 {:.1}s", p.action.display(), p.started.elapsed().as_secs_f32()));
                    self.sleep_ms(200);
                    continue;
                }
                if p.started.elapsed() >= Duration::from_secs(12) {
                    emit_log(&self.app, "WARN", "pending", format!("{}等待超时，清除动作锁并重试当前路线", p.action.display()));
                    pending = None;
                } else {
                    self.sleep_ms(200);
                    continue;
                }
            }

            if current_pos != state.pos {
                let before = state.pos;
                state = apply_progress(state, before, current_pos, lucky_level);
                if let Some(shield) = shield_read { state.shield = shield; }
                if let Some(message) = strategy.on_meter_sync(current_pos) { emit_log(&self.app, "INFO", "strategy", message); }
                emit_log(&self.app, "INFO", "meter", format!("按画面同步米数：{}m -> {}m", before, current_pos));
            }

            if state.pos >= TARGET_POS {
                emit_log(&self.app, "INFO", "finish", format!("达到 {}m，进入无米数素材收尾模式", state.pos));
                return self.run_post_200(state);
            }

            let mut action = strategy.action(state).unwrap_or_else(|| {
                if state.lucky > 0 {
                    Action::Lucky
                } else if state.boost > 0 {
                    Action::Boost
                } else {
                    Action::Normal
                }
            });
            if !self.has_resource(state, action) {
                emit_log(&self.app, "WARN", "strategy", format!("{}资源不足，使用普通奔跑继续", action.display()));
                action = Action::Normal;
            }
            emit_log(&self.app, "INFO", "action", format!("点击{}：{}m，保护={}，助跑={}，幸运={}", action.display(), state.pos, state.shield, state.boost, state.lucky));
            match self.tap_action(&image, action)? {
                TapOutcome::Confirmed => {
                    pending = Some(PendingAction { action, before: state, started: Instant::now() });
                    no_change = 0;
                }
                TapOutcome::Unavailable => {
                    match action { Action::Shield => state.shield = 0, Action::Boost => state.boost = 0, Action::Lucky => state.lucky = 0, Action::Normal => {} }
                    emit_log(&self.app, "WARN", "action", format!("{}按钮无法进入选中状态，内部资源修正为0后重新决策", action.display()));
                }
                TapOutcome::Retry => {
                    no_change += 1;
                    emit_log(&self.app, "WARN", "action", format!("{}未确认选中，保持当前步骤重试", action.display()));
                }
            }
            self.sleep_ms(self.settings.runner.fast_poll_interval_ms);
        }
    }

    fn run_post_200(&mut self, mut state: GameState) -> Result<bool> {
        self.set_phase("200m后收尾");
        emit_log(&self.app, "INFO", "finish", "200m 后不再读取米数，只关闭奖励页并等待高置信 cutdown");
        let mut first = true;
        loop {
            if self.stopped() { return Ok(false); }
            let mut image = self.controller.capture()?;
            let (page, score) = self.reader.read_page(&image);
            if page == "get_reward_page" {
                emit_log(&self.app, "INFO", "finish", format!("200m后识别到奖励页 score={:.2}，关闭", score));
                if let Some(closed) = self.close_reward_page(&image)? { image = closed; } else { continue; }
            } else if page == "howto_get_drink_page" {
                emit_log(&self.app, "STOP", "finish", "没有可用饮料，停止脚本");
                return Ok(false);
            }

            if self.confirm_cutdown(&image, self.settings.runner.post200_cutdown_threshold, self.settings.runner.post200_cutdown_confirm_hits)? {
                emit_log(&self.app, "INFO", "finish", "连续确认 cutdown，点击并等待主页面恢复");
                if self.dismiss_cutdown(&image, self.settings.runner.post200_cutdown_dismiss_timeout_ms)? {
                    emit_log(&self.app, "INFO", "finish", "cutdown 已消失，开始下一轮");
                    return Ok(true);
                }
                emit_log(&self.app, "WARN", "finish", "点击 cutdown 后主页面未恢复，继续收尾循环");
                continue;
            }

            if first && state.boost > 0 {
                emit_log(&self.app, "INFO", "finish", "200m后先尝试助跑一次");
                if matches!(self.tap_action(&image, Action::Boost)?, TapOutcome::Confirmed) { state.boost -= 1; }
                first = false;
            } else {
                let (x, y) = self.reader.click_point(&image, "btn_run")?;
                self.controller.tap(x, y)?;
                emit_log(&self.app, "INFO", "finish", "200m后点击普通奔跑；不判断米数");
                first = false;
            }
            self.sleep_ms(self.settings.runner.action_step_timeout_ms);
        }
    }

    fn tap_action(&self, image: &DynamicImage, action: Action) -> Result<TapOutcome> {
        if action == Action::Normal {
            let (x, y) = self.reader.click_point(image, "btn_run")?;
            self.controller.tap(x, y)?;
            return Ok(TapOutcome::Confirmed);
        }
        let region = action.key();
        let button = match action { Action::Shield => "btn_shield", Action::Boost => "btn_boost", Action::Lucky => "btn_lucky", _ => region };
        let mut best_score = 0.0f32;
        for attempt in 1..=self.settings.runner.skill_select_retry_limit {
            let current = if attempt == 1 { image.clone() } else { self.controller.capture()? };
            let (x, y) = self.reader.click_point(&current, button)?;
            self.controller.tap(x, y)?;
            self.sleep_ms(self.settings.runner.skill_select_delay_ms);
            let selected = self.controller.capture()?;
            let (_, score, name) = self.reader.read_skill_cancel(&selected, action.key());
            best_score = best_score.max(score);
            let formal = score >= self.settings.runner.skill_cancel_threshold;
            let soft = score >= self.settings.runner.skill_soft_confirm_threshold;
            if formal || soft {
                emit_log(&self.app, "INFO", "action", format!("{}确认选中：{} score={:.2}{}", action.display(), name, score, if soft && !formal { "（弱确认）" } else { "" }));
                let (rx, ry) = self.reader.click_point(&selected, "btn_run")?;
                self.controller.tap(rx, ry)?;
                return Ok(TapOutcome::Confirmed);
            }
            emit_log(&self.app, "WARN", "action", format!("{}未确认选中：score={:.2}，重试 {}/{}", action.display(), score, attempt, self.settings.runner.skill_select_retry_limit));
            self.sleep_ms(self.settings.runner.skill_select_retry_delay_ms);
        }
        if best_score <= self.settings.runner.unavailable_score_threshold { Ok(TapOutcome::Unavailable) } else { Ok(TapOutcome::Retry) }
    }

    fn settle_success(&self, mut before: GameState, action: Action, detected_pos: i32, lucky_level: i32) -> GameState {
        match action {
            Action::Shield => before.shield = (before.shield - 1).max(0),
            Action::Boost => before.boost = (before.boost - 1).max(0),
            Action::Lucky => before.lucky = (before.lucky - 1).max(0),
            Action::Normal => {}
        }
        let before_pos = before.pos;
        apply_progress(before, before_pos, detected_pos, lucky_level)
    }

    fn has_resource(&self, state: GameState, action: Action) -> bool {
        match action { Action::Shield => state.shield > 0, Action::Boost => state.boost > 0, Action::Lucky => state.lucky > 0, Action::Normal => true }
    }

    fn prepare_main_page(&self) -> Result<DynamicImage> {
        for _ in 0..3 {
            let image = self.controller.capture()?;
            if self.reader.main_page_ok(&image) { return Ok(image); }
            let (page, _) = self.reader.read_page(&image);
            if page == "get_reward_page" {
                if let Some(closed) = self.close_reward_page(&image)? { if self.reader.main_page_ok(&closed) { return Ok(closed); } }
            }
            if self.confirm_cutdown(&image, 0.90, 2)? && self.dismiss_cutdown(&image, 1_500)? { continue; }
            self.sleep_ms(300);
        }
        bail!("未识别到完整小游戏主页面")
    }

    fn handle_reward_if_present(&self, image: DynamicImage) -> Result<Option<DynamicImage>> {
        if self.reader.read_page(&image).0 == "get_reward_page" { self.close_reward_page(&image) } else { Ok(Some(image)) }
    }

    fn close_reward_page(&self, image: &DynamicImage) -> Result<Option<DynamicImage>> {
        let (x, y) = self.reader.click_point(image, "btn_reward_close")?;
        self.controller.tap(x, y)?;
        let deadline = Instant::now() + Duration::from_millis(self.settings.runner.followup_step_timeout_ms);
        while Instant::now() < deadline && !self.stopped() {
            let current = self.controller.capture()?;
            if self.reader.read_page(&current).0 != "get_reward_page" { return Ok(Some(current)); }
            self.sleep_ms(self.settings.runner.fast_poll_interval_ms);
        }
        Ok(None)
    }

    fn confirm_cutdown(&self, first_image: &DynamicImage, threshold: f32, hits: usize) -> Result<bool> {
        let mut confirmed = 0usize;
        let mut image = first_image.clone();
        for index in 0..hits.max(1) {
            let (ok, score, _, _) = self.reader.read_cutdown(&image, threshold);
            if !ok {
                if score >= 0.50 { emit_log(&self.app, "INFO", "cutdown", format!("疑似 cutdown score={:.2}，阈值={:.2}", score, threshold)); }
                return Ok(false);
            }
            confirmed += 1;
            if index + 1 < hits {
                self.sleep_ms(self.settings.runner.post200_cutdown_confirm_interval_ms);
                image = self.controller.capture()?;
            }
        }
        Ok(confirmed >= hits.max(1))
    }

    fn dismiss_cutdown(&self, image: &DynamicImage, timeout_ms: u64) -> Result<bool> {
        let (_, _, x, y) = self.reader.read_cutdown(image, 0.0);
        self.controller.tap(x, y)?;
        let deadline = Instant::now() + Duration::from_millis(timeout_ms);
        while Instant::now() < deadline && !self.stopped() {
            let current = self.controller.capture()?;
            if self.reader.main_page_ok(&current) { return Ok(true); }
            self.sleep_ms(100);
        }
        Ok(false)
    }

    fn read_level_or_default(&self, image: &DynamicImage) -> i32 {
        let result = self.reader.read_lucky_level(image);
        if let Some(level) = result.value {
            emit_log(&self.app, "INFO", "recognition", format!("识别到幸运等级 LV{} score={:.2}", level, result.score));
            level
        } else {
            emit_log(&self.app, "WARN", "recognition", format!("幸运等级识别失败：best={} score={:.2}，按 LV5 执行", result.name, result.score));
            5
        }
    }

    fn update_game_status(&self, state: &GameState, level: i32, phase: &str) {
        let mut status = self.status.lock();
        status.running = true;
        status.phase = phase.to_string();
        status.pos = Some(state.pos);
        status.shield = Some(state.shield);
        status.boost = Some(state.boost);
        status.lucky = Some(state.lucky);
        status.lucky_level = Some(level);
        status.strategy = self.settings.strategy_mode.clone();
        emit_status(&self.app, &status);
    }

    fn set_phase(&self, phase: &str) {
        let mut status = self.status.lock();
        status.phase = phase.to_string();
        emit_status(&self.app, &status);
    }

    fn stopped(&self) -> bool { self.stop.load(Ordering::SeqCst) }
    fn sleep_ms(&self, millis: u64) {
        let end = Instant::now() + Duration::from_millis(millis);
        while Instant::now() < end && !self.stopped() { thread::sleep(Duration::from_millis(25)); }
    }
}
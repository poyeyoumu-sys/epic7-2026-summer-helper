use crate::config::StrategyMode;
use super::{core::DynamicPlanner, model::{Action, GameState}};

const FIXED_ROUTE: [Action; 10] = [
    Action::Normal, Action::Normal, Action::Boost, Action::Shield, Action::Shield,
    Action::Lucky, Action::Shield, Action::Boost, Action::Lucky, Action::Lucky,
];
const NOMINAL_POSITIONS: [i32; 10] = [0, 10, 20, 50, 60, 70, 100, 110, 140, 170];

pub enum StrategyRuntime {
    Dynamic(DynamicPlanner),
    Fixed(Fixed200Strategy),
}

impl StrategyRuntime {
    pub fn new(mode: StrategyMode, lucky_level: i32, pos: i32) -> Self {
        match mode {
            StrategyMode::EquipmentScore => Self::Dynamic(DynamicPlanner::new(lucky_level)),
            StrategyMode::Reward32Fixed => Self::Fixed(Fixed200Strategy::from_pos(pos)),
        }
    }

    pub fn action(&mut self, state: GameState) -> Option<Action> {
        match self {
            Self::Dynamic(planner) => planner.best_action(state),
            Self::Fixed(fixed) => fixed.get_action(state),
        }
    }

    pub fn on_success(&mut self, action: Action, before: i32, after: i32) -> Option<String> {
        match self { Self::Fixed(f) => f.on_success(action, before, after), _ => None }
    }

    pub fn on_shield_failure(&mut self, before: i32, shield_left: i32) -> Option<String> {
        match self { Self::Fixed(f) => Some(f.on_shield_failure(before, shield_left)), _ => None }
    }

    pub fn on_meter_sync(&mut self, pos: i32) -> Option<String> {
        match self { Self::Fixed(f) => f.on_meter_sync(pos), _ => None }
    }
}

#[derive(Debug, Clone)]
pub struct Fixed200Strategy {
    route_index: usize,
    delay_until_pos: Option<i32>,
    last_action_is_delay: bool,
    boost_success_count: usize,
    shifted: bool,
}

impl Fixed200Strategy {
    pub fn from_pos(pos: i32) -> Self {
        let mut index = 0;
        for (i, nominal) in NOMINAL_POSITIONS.iter().enumerate() { if pos >= *nominal { index = i; } }
        Self { route_index: index.min(FIXED_ROUTE.len()-1), delay_until_pos: None, last_action_is_delay: false, boost_success_count: usize::from(pos >= 50), shifted: false }
    }

    fn step_text(&self) -> String {
        FIXED_ROUTE.get(self.route_index).map(|a| format!("第{}步 {}", self.route_index+1, a.display())).unwrap_or_else(|| "固定路线已执行完".into())
    }

    fn skip_shield_and_delay(&mut self, pos: i32, reason: &str) -> String {
        let old = self.step_text();
        if matches!(FIXED_ROUTE.get(self.route_index), Some(Action::Shield)) { self.route_index += 1; }
        self.delay_until_pos = Some(pos + 10);
        self.last_action_is_delay = true;
        self.shifted = true;
        format!("200米优先：{}，跳过保护位（{}），普通跑到 {}m 后继续 {}", reason, old, pos+10, self.step_text())
    }

    pub fn get_action(&mut self, state: GameState) -> Option<Action> {
        if state.pos >= 200 { return None; }
        if let Some(target) = self.delay_until_pos {
            if state.pos < target { self.last_action_is_delay = true; return Some(Action::Normal); }
            self.delay_until_pos = None;
            self.last_action_is_delay = false;
        }
        if self.route_index >= FIXED_ROUTE.len() {
            return Some(if state.lucky > 0 { Action::Lucky } else if state.boost > 0 { Action::Boost } else { Action::Normal });
        }
        let action = FIXED_ROUTE[self.route_index];
        if action == Action::Shield && state.shield <= 0 {
            self.skip_shield_and_delay(state.pos, "当前保护次数为0");
            return Some(Action::Normal);
        }
        self.last_action_is_delay = false;
        Some(action)
    }

    pub fn on_success(&mut self, action: Action, before: i32, after: i32) -> Option<String> {
        if self.last_action_is_delay {
            if self.delay_until_pos.map(|p| after >= p).unwrap_or(false) {
                let target = self.delay_until_pos.take().unwrap_or(after);
                self.last_action_is_delay = false;
                return Some(format!("200米优先补位完成：{}m -> {}m，达到 {}m，继续 {}", before, after, target, self.step_text()));
            }
            return Some(format!("200米优先补位中：{}m -> {}m", before, after));
        }
        if FIXED_ROUTE.get(self.route_index) == Some(&action) {
            if action == Action::Boost { self.boost_success_count += 1; }
            self.route_index += 1;
            return Some(format!("200米优先路线推进：完成 {}，下一步 {}", action.display(), self.step_text()));
        }
        None
    }

    pub fn on_meter_sync(&mut self, pos: i32) -> Option<String> {
        if let Some(target) = self.delay_until_pos {
            if pos >= target {
                self.delay_until_pos = None;
                self.last_action_is_delay = false;
                return Some(format!("200米优先补位完成：当前 {}m，继续 {}", pos, self.step_text()));
            }
            return None;
        }
        if self.shifted { return None; }
        let old = self.route_index;
        let max_index = if self.boost_success_count == 0 { 2 } else { FIXED_ROUTE.len()-1 };
        for (i, nominal) in NOMINAL_POSITIONS.iter().enumerate().take(max_index+1) {
            if pos >= *nominal { self.route_index = self.route_index.max(i); }
        }
        (self.route_index > old).then(|| format!("200米优先米数同步：当前 {}m，对齐到 {}", pos, self.step_text()))
    }

    pub fn on_shield_failure(&mut self, before: i32, shield_left: i32) -> String {
        let index = self.route_index;
        let step = self.step_text();
        if index == 3 || index == 4 {
            return self.skip_shield_and_delay(before, &format!("{} 防护失败，50/60m 前段保护位只尝试一次", step));
        }
        if index == 6 && shield_left > 0 {
            self.shifted = true;
            self.delay_until_pos = None;
            self.last_action_is_delay = false;
            return format!("200米优先：{}m {} 防护失败，剩余保护={}，100m 后段保护位原地重试", before, step, shield_left);
        }
        self.skip_shield_and_delay(before, &format!("{} 防护失败且保护已用完", step))
    }
}

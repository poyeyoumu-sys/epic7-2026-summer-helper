use std::collections::HashMap;
use super::model::{Action, GameState, lucky_config};

pub const TARGET_POS: i32 = 200;

const PROB: &[(i32, f64)] = &[
    (0,1.00),(10,0.96),(20,0.92),(30,0.88),(40,0.84),(50,0.80),(60,0.75),(70,0.70),(80,0.65),(90,0.60),
    (100,0.75),(110,0.69),(120,0.63),(130,0.56),(140,0.50),
    (150,0.70),(160,0.64),(170,0.58),(180,0.51),(190,0.45),
];

#[derive(Debug, Clone, Copy)]
pub struct Transition {
    pub probability: f64,
    pub success: Option<GameState>,
    pub failure: Option<GameState>,
    pub success_pos: i32,
    pub immediate_reward: f64,
}

pub fn probability(pos: i32) -> Option<f64> { PROB.iter().find(|(p,_)| *p == pos).map(|(_,v)| *v) }

pub fn apply_progress(mut state: GameState, before: i32, after: i32, lucky_level: i32) -> GameState {
    state.pos = after;
    if before < 100 && after >= 100 && !state.got100 {
        state.shield = (state.shield + 2).min(4);
        state.boost = (state.boost + 1).min(2);
        state.got100 = true;
    }
    let lucky = lucky_config(lucky_level);
    if lucky.refill_meter > 0 && before < lucky.refill_meter && after >= lucky.refill_meter && !state.got_lucky_refill {
        state.lucky = (state.lucky + 1).min(lucky.max);
        state.got_lucky_refill = true;
    }
    if before < 200 && after >= 200 && !state.got200 {
        state.shield = (state.shield + 2).min(4);
        state.boost = (state.boost + 1).min(2);
        state.got200 = true;
    }
    state
}

pub fn transition(state: GameState, action: Action, lucky_level: i32) -> Option<Transition> {
    if state.pos >= TARGET_POS { return None; }
    let (success_pos, p, mut used) = match action {
        Action::Normal => (state.pos + 10, probability(state.pos)?, state),
        Action::Shield if state.pos > 0 && state.shield > 0 => {
            let mut s = state; s.shield -= 1; (state.pos + 10, probability(state.pos)?, s)
        }
        Action::Boost if state.pos > 0 && state.boost > 0 => {
            let mut s = state; s.boost -= 1; (state.pos + 30, probability((state.pos - 10).max(0))?, s)
        }
        Action::Lucky if state.pos > 0 && state.lucky > 0 => {
            let mut s = state; s.lucky -= 1; (state.pos + 30, 1.0, s)
        }
        _ => return None,
    };
    let reward = (if state.pos < 100 && success_pos >= 100 { 3.0 } else { 0.0 })
        + (if state.pos < 200 && success_pos >= 200 { 4.0 } else { 0.0 });
    let success = if success_pos >= TARGET_POS { None } else { Some(apply_progress(used, state.pos, success_pos, lucky_level)) };
    let failure = match action {
        Action::Shield => Some(used),
        Action::Lucky => None,
        _ => None,
    };
    Some(Transition { probability: p, success, failure, success_pos, immediate_reward: reward })
}

#[derive(Debug, Clone, Copy, Default)]
struct Value { score: f64, cost: f64 }

pub struct DynamicPlanner {
    lucky_level: i32,
    memo: HashMap<GameState, Value>,
}

impl DynamicPlanner {
    pub fn new(lucky_level: i32) -> Self { Self { lucky_level, memo: HashMap::new() } }

    pub fn best_action(&mut self, state: GameState) -> Option<Action> {
        let mut best: Option<(Action, Value)> = None;
        for action in [Action::Normal, Action::Shield, Action::Boost, Action::Lucky] {
            let Some(value) = self.action_value(state, action) else { continue; };
            let better = best.map(|(_,b)| value.score > b.score + 1e-9 || ((value.score-b.score).abs() <= 1e-9 && value.cost < b.cost)).unwrap_or(true);
            if better { best = Some((action, value)); }
        }
        best.map(|x| x.0)
    }

    fn evaluate(&mut self, state: GameState) -> Value {
        if state.pos >= TARGET_POS { return Value::default(); }
        if let Some(value) = self.memo.get(&state) { return *value; }
        let mut best = Value { score: -1.0, cost: f64::INFINITY };
        for action in [Action::Normal, Action::Shield, Action::Boost, Action::Lucky] {
            if let Some(value) = self.action_value(state, action) {
                if value.score > best.score + 1e-9 || ((value.score-best.score).abs() <= 1e-9 && value.cost < best.cost) { best = value; }
            }
        }
        if best.score < 0.0 { best = Value::default(); }
        self.memo.insert(state, best);
        best
    }

    fn action_value(&mut self, state: GameState, action: Action) -> Option<Value> {
        let tr = transition(state, action, self.lucky_level)?;
        let success_future = tr.success.map(|s| self.evaluate(s)).unwrap_or_default();
        let failure_future = tr.failure.map(|s| self.evaluate(s)).unwrap_or_default();
        Some(Value {
            score: tr.probability * (tr.immediate_reward + success_future.score) + (1.0-tr.probability) * failure_future.score,
            cost: action.cost() + tr.probability * success_future.cost + (1.0-tr.probability) * failure_future.cost,
        })
    }
}

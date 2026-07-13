use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GameState {
    pub pos: i32,
    pub shield: i32,
    pub boost: i32,
    pub lucky: i32,
    pub got100: bool,
    pub got200: bool,
    pub got_lucky_refill: bool,
}

impl GameState {
    pub fn manual(pos: i32, shield: i32, boost: i32, lucky: i32, lucky_level: i32) -> Self {
        let refill = lucky_config(lucky_level).refill_meter;
        Self {
            pos,
            shield: shield.clamp(0, 4),
            boost: boost.clamp(0, 2),
            lucky: lucky.clamp(0, lucky_config(lucky_level).max),
            got100: pos >= 100,
            got200: pos >= 200,
            got_lucky_refill: refill > 0 && pos >= refill,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action { Normal, Shield, Boost, Lucky }

impl Action {
    pub fn key(self) -> &'static str { match self { Self::Normal => "normal", Self::Shield => "shield", Self::Boost => "boost", Self::Lucky => "lucky" } }
    pub fn display(self) -> &'static str { match self { Self::Normal => "普通奔跑", Self::Shield => "防护", Self::Boost => "助跑", Self::Lucky => "超级幸运" } }
    pub fn cost(self) -> f64 { match self { Self::Lucky => 3.0, _ => 1.0 } }
}

#[derive(Debug, Clone, Copy)]
pub struct LuckyLevelConfig { pub start: i32, pub max: i32, pub refill_meter: i32 }

pub fn lucky_config(level: i32) -> LuckyLevelConfig {
    match level {
        0 => LuckyLevelConfig { start: 0, max: 0, refill_meter: 0 },
        1 => LuckyLevelConfig { start: 1, max: 1, refill_meter: 200 },
        2 => LuckyLevelConfig { start: 1, max: 1, refill_meter: 180 },
        3 => LuckyLevelConfig { start: 1, max: 2, refill_meter: 180 },
        4 => LuckyLevelConfig { start: 1, max: 2, refill_meter: 150 },
        _ => LuckyLevelConfig { start: 2, max: 2, refill_meter: 150 },
    }
}

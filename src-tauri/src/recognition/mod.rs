mod matcher;

use anyhow::{Context, Result};
use image::{DynamicImage, GenericImageView};
use serde::Deserialize;
use std::{collections::HashMap, fs, path::{Path, PathBuf}};

use matcher::similarity;

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct Region { pub x: i32, pub y: i32, pub w: i32, pub h: i32 }

#[derive(Debug, Clone, Deserialize)]
struct Resolution { width: i32, height: i32 }

#[derive(Debug, Clone, Deserialize)]
struct RecognitionConfigFile {
    reference_resolution: Resolution,
    min_confidence: f32,
    regions: HashMap<String, Region>,
}

#[derive(Clone)]
pub struct StateReader {
    reference_width: i32,
    reference_height: i32,
    min_confidence: f32,
    regions: HashMap<String, Region>,
    templates: HashMap<String, DynamicImage>,
}

#[derive(Debug, Clone)]
pub struct MatchResult<T> {
    pub value: Option<T>,
    pub score: f32,
    pub name: String,
}

impl StateReader {
    pub fn load(config_path: &Path, template_dir: &Path) -> Result<Self> {
        let config: RecognitionConfigFile = serde_json::from_str(&fs::read_to_string(config_path)
            .with_context(|| format!("读取识别配置失败：{}", config_path.display()))?)?;
        let mut templates = HashMap::new();
        for entry in fs::read_dir(template_dir).with_context(|| format!("读取模板目录失败：{}", template_dir.display()))? {
            let path = entry?.path();
            if path.extension().and_then(|s| s.to_str()).map(|s| s.eq_ignore_ascii_case("png")) != Some(true) { continue; }
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                if let Ok(image) = image::open(&path) { templates.insert(stem.to_string(), image); }
            }
        }
        Ok(Self {
            reference_width: config.reference_resolution.width,
            reference_height: config.reference_resolution.height,
            min_confidence: config.min_confidence,
            regions: config.regions,
            templates,
        })
    }

    pub fn click_point(&self, image: &DynamicImage, region: &str) -> Result<(i32, i32)> {
        let r = self.scaled_region(image, region)?;
        Ok((r.x + r.w / 2, r.y + r.h / 2))
    }

    pub fn main_page_ok(&self, image: &DynamicImage) -> bool {
        ["btn_run", "btn_shield", "btn_boost", "btn_lucky"].iter()
            .all(|name| self.match_named(image, name, name).map(|r| r.score >= 0.55).unwrap_or(false))
    }

    pub fn read_meter(&self, image: &DynamicImage) -> MatchResult<i32> {
        let names = self.templates.keys().filter(|name| name.starts_with("current_meter")).cloned().collect::<Vec<_>>();
        let best = self.match_candidates(image, "current_meter", &names);
        let value = best.as_ref().and_then(|(name, score)| {
            if *score < self.min_confidence { return None; }
            name.trim_start_matches("current_meter").parse::<i32>().ok()
        });
        MatchResult { value, score: best.as_ref().map(|x| x.1).unwrap_or(0.0), name: best.map(|x| x.0).unwrap_or_default() }
    }

    pub fn read_lucky_level(&self, image: &DynamicImage) -> MatchResult<i32> {
        let names = (1..=5).map(|v| format!("lucky_lv{}", v)).collect::<Vec<_>>();
        let best = self.match_candidates(image, "lucky_level", &names);
        let value = best.as_ref().and_then(|(name, score)| {
            if *score < self.min_confidence { None } else { name.trim_start_matches("lucky_lv").parse().ok() }
        });
        MatchResult { value, score: best.as_ref().map(|x| x.1).unwrap_or(0.0), name: best.map(|x| x.0).unwrap_or_default() }
    }

    pub fn read_shield(&self, image: &DynamicImage) -> MatchResult<i32> {
        let names = (0..=4).map(|v| format!("shield_count{}", v)).collect::<Vec<_>>();
        let best = self.match_candidates(image, "shield_count", &names);
        let value = best.as_ref().and_then(|(name, score)| {
            if *score < self.min_confidence { None } else { name.trim_start_matches("shield_count").parse().ok() }
        });
        MatchResult { value, score: best.as_ref().map(|x| x.1).unwrap_or(0.0), name: best.map(|x| x.0).unwrap_or_default() }
    }

    pub fn read_page(&self, image: &DynamicImage) -> (String, f32) {
        let reward = self.match_named(image, "get_reward_page", "get_reward_page").map(|r| r.score).unwrap_or(0.0);
        let drink = self.match_named(image, "howto_get_drink_page", "howto_get_drink_page").map(|r| r.score).unwrap_or(0.0);
        if reward >= self.min_confidence && reward >= drink { ("get_reward_page".into(), reward) }
        else if drink >= self.min_confidence { ("howto_get_drink_page".into(), drink) }
        else { ("main_game".into(), reward.max(drink)) }
    }

    pub fn read_cutdown(&self, image: &DynamicImage, threshold: f32) -> (bool, f32, i32, i32) {
        let score = self.match_named(image, "cutdown", "cutdown").map(|r| r.score).unwrap_or(0.0);
        let (x, y) = self.click_point(image, "cutdown").unwrap_or((0, 0));
        (score >= threshold, score, x, y)
    }

    pub fn read_skill_cancel(&self, image: &DynamicImage, action: &str) -> (bool, f32, String) {
        let name = match action { "shield" => "btn_shield_cancel", "boost" => "btn_boost_cancel", "lucky" => "btn_lucky_cancel", _ => return (false, 0.0, String::new()) };
        let score = self.match_named(image, name, name).map(|r| r.score).unwrap_or(0.0);
        (score >= self.min_confidence, score, name.to_string())
    }

    pub fn match_button_score(&self, image: &DynamicImage, name: &str) -> f32 {
        self.match_named(image, name, name).map(|r| r.score).unwrap_or(0.0)
    }

    fn match_candidates(&self, image: &DynamicImage, region: &str, names: &[String]) -> Option<(String, f32)> {
        names.iter().filter_map(|name| self.match_named(image, region, name).ok().map(|result| (name.clone(), result.score)))
            .max_by(|a, b| a.1.total_cmp(&b.1))
    }

    fn match_named(&self, image: &DynamicImage, region: &str, template: &str) -> Result<MatchResult<()>> {
        let crop = self.crop(image, region)?;
        let tpl = self.templates.get(template).with_context(|| format!("缺少模板：{}", template))?;
        let score = similarity(&crop, tpl);
        Ok(MatchResult { value: Some(()), score, name: template.to_string() })
    }

    fn crop(&self, image: &DynamicImage, name: &str) -> Result<DynamicImage> {
        let r = self.scaled_region(image, name)?;
        Ok(image.crop_imm(r.x as u32, r.y as u32, r.w as u32, r.h as u32))
    }

    fn scaled_region(&self, image: &DynamicImage, name: &str) -> Result<Region> {
        let base = *self.regions.get(name).with_context(|| format!("缺少识别区域：{}", name))?;
        let (width, height) = image.dimensions();
        let sx = width as f32 / self.reference_width as f32;
        let sy = height as f32 / self.reference_height as f32;
        let mut r = Region {
            x: (base.x as f32 * sx).round() as i32,
            y: (base.y as f32 * sy).round() as i32,
            w: (base.w as f32 * sx).round().max(1.0) as i32,
            h: (base.h as f32 * sy).round().max(1.0) as i32,
        };
        r.x = r.x.clamp(0, width.saturating_sub(1) as i32);
        r.y = r.y.clamp(0, height.saturating_sub(1) as i32);
        r.w = r.w.min(width as i32 - r.x).max(1);
        r.h = r.h.min(height as i32 - r.y).max(1);
        Ok(r)
    }
}

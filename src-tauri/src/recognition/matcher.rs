use image::{imageops::{resize, FilterType}, DynamicImage};

pub fn similarity(crop: &DynamicImage, template: &DynamicImage) -> f32 {
    let crop = crop.to_luma8();
    let template = resize(&template.to_luma8(), crop.width(), crop.height(), FilterType::Triangle);
    if crop.is_empty() || template.is_empty() { return 0.0; }
    let n = (crop.width() * crop.height()) as f64;
    let mean_a = crop.pixels().map(|p| p[0] as f64).sum::<f64>() / n;
    let mean_b = template.pixels().map(|p| p[0] as f64).sum::<f64>() / n;
    let mut numerator = 0.0;
    let mut denom_a = 0.0;
    let mut denom_b = 0.0;
    let mut mae = 0.0;
    for (a, b) in crop.pixels().zip(template.pixels()) {
        let da = a[0] as f64 - mean_a;
        let db = b[0] as f64 - mean_b;
        numerator += da * db;
        denom_a += da * da;
        denom_b += db * db;
        mae += (a[0] as f64 - b[0] as f64).abs();
    }
    let denom = (denom_a * denom_b).sqrt();
    let ncc = if denom > 1e-9 { (numerator / denom).clamp(-1.0, 1.0) } else { 0.0 };
    let mae_score = 1.0 - (mae / n / 255.0);
    (((ncc + 1.0) * 0.5) * 0.80 + mae_score * 0.20).clamp(0.0, 1.0) as f32
}

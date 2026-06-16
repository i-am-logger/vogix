//! Color extraction from theme palettes.
//!
//! Uses praxis::science::colors for color parsing and analysis,
//! then produces a fully-saturated tint color for the GLSL screen shader.

use pr4xis_domains::applied::hmi::theming::ontology::Palette;
#[cfg(test)]
use pr4xis_domains::natural::colors::Rgb;

/// Normalized RGB color with components in [0.0, 1.0] for GLSL embedding.
#[derive(Debug, Clone, PartialEq)]
pub struct ShaderColor {
    pub r: f32,
    pub g: f32,
    pub b: f32,
}

impl ShaderColor {
    pub const fn new(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b }
    }

    pub const WHITE: Self = Self::new(1.0, 1.0, 1.0);

    /// Create from praxis Rgb (u8 channels → f32 normalized).
    #[cfg(test)]
    pub fn from_rgb(rgb: &Rgb) -> Self {
        Self::new(
            rgb.r as f32 / 255.0,
            rgb.g as f32 / 255.0,
            rgb.b as f32 / 255.0,
        )
    }
}

/// Extract the dominant hue from the monochromatic ramp and return a vivid tint.
///
/// The ramp = the background/foreground slots (base00..07), selected by ontology
/// role rather than by raw `base0X` string keys — so every scheme contributes,
/// including ansi16 (whose loader emits `color00`/`background`, never `base00`,
/// and previously yielded a white/no-hue tint). Circular saturation-weighted hue
/// mean finds the dominant cast; returns white if the ramp is achromatic.
pub fn extract_shader_color(palette: &Palette) -> ShaderColor {
    use pr4xis::category::FinitelyGenerated;
    use pr4xis_domains::applied::hmi::theming::base16::{ColorSlot, SemanticRole};

    let mut sum_sin = 0.0_f64;
    let mut sum_cos = 0.0_f64;
    let mut total_weight = 0.0_f64;

    for slot in ColorSlot::variants() {
        if !matches!(
            slot.role(),
            SemanticRole::Background | SemanticRole::Foreground
        ) {
            continue;
        }
        let Some(rgb) = palette.get(&slot) else {
            continue;
        };

        // Use praxis Rgb methods for hue and saturation
        let sat = rgb.saturation();
        if sat < 0.01 {
            continue;
        }

        let Some(hue_deg) = rgb.hue() else {
            continue;
        };

        let angle = (hue_deg / 360.0) * std::f64::consts::TAU;
        sum_sin += sat * angle.sin();
        sum_cos += sat * angle.cos();
        total_weight += sat;
    }

    if total_weight < 0.05 {
        return ShaderColor::WHITE;
    }

    let avg_angle = sum_sin.atan2(sum_cos);
    let avg_hue = (avg_angle / std::f64::consts::TAU).rem_euclid(1.0) as f32;

    // Fully saturated color at the extracted hue (HSL S=1.0, L=0.5 = max vivid)
    let (r, g, b) = hsl_to_rgb(avg_hue, 1.0, 0.5);
    ShaderColor::new(r, g, b)
}

/// Convert HSL to RGB. All inputs in [0..1].
fn hsl_to_rgb(h: f32, s: f32, l: f32) -> (f32, f32, f32) {
    if s < 0.001 {
        return (l, l, l);
    }

    let q = if l < 0.5 {
        l * (1.0 + s)
    } else {
        l + s - l * s
    };
    let p = 2.0 * l - q;

    let hue_to_rgb = |t: f32| -> f32 {
        let t = t.rem_euclid(1.0);
        if t < 1.0 / 6.0 {
            p + (q - p) * 6.0 * t
        } else if t < 1.0 / 2.0 {
            q
        } else if t < 2.0 / 3.0 {
            p + (q - p) * (2.0 / 3.0 - t) * 6.0
        } else {
            p
        }
    };

    (
        hue_to_rgb(h + 1.0 / 3.0),
        hue_to_rgb(h),
        hue_to_rgb(h - 1.0 / 3.0),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    use pr4xis_domains::applied::hmi::theming::base16::ColorSlot;

    fn empty_palette() -> Palette {
        Palette::new()
    }

    fn ramp_palette(hexes: [&str; 8]) -> Palette {
        let slots = [
            ColorSlot::Base00,
            ColorSlot::Base01,
            ColorSlot::Base02,
            ColorSlot::Base03,
            ColorSlot::Base04,
            ColorSlot::Base05,
            ColorSlot::Base06,
            ColorSlot::Base07,
        ];
        let mut p = Palette::new();
        for (slot, hex) in slots.into_iter().zip(hexes) {
            p.insert(slot, Rgb::from_hex(hex).unwrap());
        }
        p
    }

    fn green_palette() -> Palette {
        ramp_palette([
            "#1a2b1a", "#2a3b2a", "#3a4b3a", "#4a5b4a", "#6a7b6a", "#8a9b8a", "#aabbaa", "#ccddcc",
        ])
    }

    #[test]
    fn test_achromatic_returns_white() {
        let mut p = Palette::new();
        p.insert(ColorSlot::Base00, Rgb::from_hex("#111111").unwrap());
        p.insert(ColorSlot::Base05, Rgb::from_hex("#cccccc").unwrap());
        assert_eq!(extract_shader_color(&p), ShaderColor::WHITE);
    }

    #[test]
    fn test_empty_returns_white() {
        assert_eq!(extract_shader_color(&empty_palette()), ShaderColor::WHITE);
    }

    #[test]
    fn test_green_palette_extracts_green_hue() {
        let color = extract_shader_color(&green_palette());
        // Green-ish: g should be dominant
        assert!(color.g > color.r);
        assert!(color.g > color.b);
    }

    #[test]
    fn test_shader_color_from_rgb() {
        let rgb = Rgb::new(128, 64, 255);
        let sc = ShaderColor::from_rgb(&rgb);
        assert!((sc.r - 128.0 / 255.0).abs() < 0.01);
        assert!((sc.g - 64.0 / 255.0).abs() < 0.01);
        assert!((sc.b - 1.0).abs() < 0.01);
    }
}

//! Color extraction from theme palettes.
//!
//! Uses praxis::science::colors for color parsing and analysis,
//! then produces a fully-saturated tint color for the GLSL screen shader.

use praxis_domains::science::colors::Rgb;
use std::collections::HashMap;

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

/// Parse a hex color string (#RRGGBB or RRGGBB) to normalized f32 RGB.
///
/// Uses praxis Rgb::from_hex for parsing.
pub fn hex_to_rgb(hex: &str) -> Option<(f32, f32, f32)> {
    let rgb = Rgb::from_hex(hex)?;
    Some((
        rgb.r as f32 / 255.0,
        rgb.g as f32 / 255.0,
        rgb.b as f32 / 255.0,
    ))
}

/// Extract the dominant hue from base00-07 colors and return a vivid shader tint color.
///
/// Uses praxis Rgb for color parsing, hue/saturation extraction.
/// Circular weighted average of hues (weighted by saturation) finds the
/// dominant color cast. Returns white if the palette is achromatic.
pub fn extract_shader_color(colors: &HashMap<String, String>) -> ShaderColor {
    let base_keys = [
        "base00", "base01", "base02", "base03", "base04", "base05", "base06", "base07",
    ];

    let mut sum_sin = 0.0_f64;
    let mut sum_cos = 0.0_f64;
    let mut total_weight = 0.0_f64;

    for key in &base_keys {
        let Some(hex) = colors.get(*key) else {
            continue;
        };
        let Some(rgb) = Rgb::from_hex(hex) else {
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

    fn empty_colors() -> HashMap<String, String> {
        HashMap::new()
    }

    fn green_palette() -> HashMap<String, String> {
        let mut c = HashMap::new();
        c.insert("base00".into(), "#1a2b1a".into());
        c.insert("base01".into(), "#2a3b2a".into());
        c.insert("base02".into(), "#3a4b3a".into());
        c.insert("base03".into(), "#4a5b4a".into());
        c.insert("base04".into(), "#6a7b6a".into());
        c.insert("base05".into(), "#8a9b8a".into());
        c.insert("base06".into(), "#aabbaa".into());
        c.insert("base07".into(), "#ccddcc".into());
        c
    }

    #[test]
    fn test_hex_to_rgb() {
        let (r, g, b) = hex_to_rgb("#ff0000").unwrap();
        assert!((r - 1.0).abs() < 0.01);
        assert!(g < 0.01);
        assert!(b < 0.01);
    }

    #[test]
    fn test_hex_to_rgb_no_hash() {
        assert!(hex_to_rgb("00ff00").is_some());
    }

    #[test]
    fn test_achromatic_returns_white() {
        let mut c = HashMap::new();
        c.insert("base00".into(), "#111111".into());
        c.insert("base05".into(), "#cccccc".into());
        let color = extract_shader_color(&c);
        assert_eq!(color, ShaderColor::WHITE);
    }

    #[test]
    fn test_empty_returns_white() {
        assert_eq!(extract_shader_color(&empty_colors()), ShaderColor::WHITE);
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

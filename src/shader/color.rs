//! Color extraction from theme palettes.
//!
//! Extracts the dominant hue from base00-07 monochromatic scale colors
//! and produces a fully-saturated tint color for the screen shader.

use std::collections::HashMap;

/// Normalized RGB color with components in [0.0, 1.0].
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

    /// White fallback for achromatic themes.
    pub const WHITE: Self = Self::new(1.0, 1.0, 1.0);
}

/// Parse a hex color string (#RRGGBB or RRGGBB) to normalized RGB.
pub fn hex_to_rgb(hex: &str) -> Option<(f32, f32, f32)> {
    let hex = hex.trim_start_matches('#');
    if hex.len() < 6 {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()? as f32 / 255.0;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()? as f32 / 255.0;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()? as f32 / 255.0;
    Some((r, g, b))
}

/// Convert RGB to HSL. Returns (hue [0..1], saturation [0..1], lightness [0..1]).
fn rgb_to_hsl(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) / 2.0;
    let d = max - min;

    if d < 0.001 {
        return (0.0, 0.0, l);
    }

    let s = if l > 0.5 {
        d / (2.0 - max - min)
    } else {
        d / (max + min)
    };

    let h = if (max - r).abs() < 0.001 {
        let mut h = (g - b) / d;
        if g < b {
            h += 6.0;
        }
        h / 6.0
    } else if (max - g).abs() < 0.001 {
        ((b - r) / d + 2.0) / 6.0
    } else {
        ((r - g) / d + 4.0) / 6.0
    };

    (h, s, l)
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

/// Extract the dominant hue from base00-07 colors and return a vivid shader tint color.
///
/// Uses circular weighted average of hues (weighted by saturation) to find the
/// dominant color cast in the monochromatic palette. Returns white if the palette
/// is achromatic (all grays).
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
        let Some((r, g, b)) = hex_to_rgb(hex) else {
            continue;
        };
        let (h, s, _l) = rgb_to_hsl(r, g, b);

        // Weight by saturation — more colorful samples have more reliable hue
        let w = s as f64;
        if w < 0.01 {
            continue;
        }

        let angle = h as f64 * std::f64::consts::TAU;
        sum_sin += w * angle.sin();
        sum_cos += w * angle.cos();
        total_weight += w;
    }

    // If total saturation is negligible, the palette is achromatic
    if total_weight < 0.05 {
        return ShaderColor::WHITE;
    }

    let avg_angle = sum_sin.atan2(sum_cos);
    let avg_hue = (avg_angle / std::f64::consts::TAU).rem_euclid(1.0) as f32;

    // Fully saturated color at the extracted hue (S=1.0, L=0.5 = max vivid)
    let (r, g, b) = hsl_to_rgb(avg_hue, 1.0, 0.5);
    ShaderColor::new(r, g, b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_to_rgb_with_hash() {
        let (r, g, b) = hex_to_rgb("#FF0000").unwrap();
        assert!((r - 1.0).abs() < 0.01);
        assert!(g < 0.01);
        assert!(b < 0.01);
    }

    #[test]
    fn hex_to_rgb_without_hash() {
        let (r, g, b) = hex_to_rgb("00FF00").unwrap();
        assert!(r < 0.01);
        assert!((g - 1.0).abs() < 0.01);
        assert!(b < 0.01);
    }

    #[test]
    fn hex_to_rgb_invalid() {
        assert!(hex_to_rgb("xyz").is_none());
        assert!(hex_to_rgb("#GG0000").is_none());
    }

    #[test]
    fn hsl_roundtrip_red() {
        let (h, s, l) = rgb_to_hsl(1.0, 0.0, 0.0);
        let (r, g, b) = hsl_to_rgb(h, s, l);
        assert!((r - 1.0).abs() < 0.01);
        assert!(g < 0.01);
        assert!(b < 0.01);
    }

    #[test]
    fn hsl_roundtrip_green() {
        let (h, s, l) = rgb_to_hsl(0.0, 1.0, 0.0);
        let (r, g, b) = hsl_to_rgb(h, s, l);
        assert!(r < 0.01);
        assert!((g - 1.0).abs() < 0.01);
        assert!(b < 0.01);
    }

    #[test]
    fn hsl_roundtrip_blue() {
        let (h, s, l) = rgb_to_hsl(0.0, 0.0, 1.0);
        let (r, g, b) = hsl_to_rgb(h, s, l);
        assert!(r < 0.01);
        assert!(g < 0.01);
        assert!((b - 1.0).abs() < 0.01);
    }

    #[test]
    fn hsl_gray_is_achromatic() {
        let (_, s, _) = rgb_to_hsl(0.5, 0.5, 0.5);
        assert!(s < 0.01);
    }

    #[test]
    fn extract_achromatic_gives_white() {
        let mut colors = HashMap::new();
        for (i, key) in [
            "base00", "base01", "base02", "base03", "base04", "base05", "base06", "base07",
        ]
        .iter()
        .enumerate()
        {
            let v = (i as f32 / 7.0 * 255.0) as u8;
            colors.insert(key.to_string(), format!("#{:02x}{:02x}{:02x}", v, v, v));
        }
        let c = extract_shader_color(&colors);
        assert_eq!(c, ShaderColor::WHITE);
    }

    #[test]
    fn extract_catppuccin_mocha_is_purple() {
        // Catppuccin Mocha base00-07 have a purple/blue hue
        let mut colors = HashMap::new();
        colors.insert("base00".into(), "#1e1e2e".into());
        colors.insert("base01".into(), "#181825".into());
        colors.insert("base02".into(), "#313244".into());
        colors.insert("base03".into(), "#45475a".into());
        colors.insert("base04".into(), "#585b70".into());
        colors.insert("base05".into(), "#cdd6f4".into());
        colors.insert("base06".into(), "#f5e0dc".into());
        colors.insert("base07".into(), "#b4befe".into());

        let c = extract_shader_color(&colors);
        // Should be in the blue-purple range: b > r, b > g
        assert!(
            c.b > c.r,
            "expected blue > red for catppuccin: r={:.2} b={:.2}",
            c.r,
            c.b
        );
        assert!(
            c.b > c.g,
            "expected blue > green for catppuccin: g={:.2} b={:.2}",
            c.g,
            c.b
        );
    }

    #[test]
    fn extract_gruvbox_is_warm() {
        // Gruvbox dark base00-07 have a warm brown/orange hue
        let mut colors = HashMap::new();
        colors.insert("base00".into(), "#282828".into());
        colors.insert("base01".into(), "#3c3836".into());
        colors.insert("base02".into(), "#504945".into());
        colors.insert("base03".into(), "#665c54".into());
        colors.insert("base04".into(), "#bdae93".into());
        colors.insert("base05".into(), "#d5c4a1".into());
        colors.insert("base06".into(), "#ebdbb2".into());
        colors.insert("base07".into(), "#fbf1c7".into());

        let c = extract_shader_color(&colors);
        // Should be warm: r > b
        assert!(
            c.r > c.b,
            "expected red > blue for gruvbox: r={:.2} b={:.2}",
            c.r,
            c.b
        );
    }

    #[test]
    fn extract_missing_keys_falls_back() {
        let colors = HashMap::new();
        let c = extract_shader_color(&colors);
        assert_eq!(c, ShaderColor::WHITE);
    }

    #[test]
    fn extract_partial_keys_works() {
        let mut colors = HashMap::new();
        colors.insert("base05".into(), "#cdd6f4".into()); // blue-ish
        let c = extract_shader_color(&colors);
        // Should still produce a color (not white) since base05 has some saturation
        assert!(c.b > 0.3, "expected some blue from single base05");
    }
}

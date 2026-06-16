//! GLSL monochromatic shader generation.
//!
//! Generates a screen-wide monochromatic tint shader for Hyprland's
//! `decoration:screen_shader`. The shader converts all screen pixels
//! to a single-hue monochromatic look derived from the current theme's
//! base00-07 color scale.

use crate::errors::{Result, VogixError};
use std::fs;
use std::path::PathBuf;

use super::color::ShaderColor;
use pr4xis_domains::applied::hmi::theming::ontology::Palette;

/// Rec. 709 luma coefficients for luminance calculation.
const LUMA_R: f32 = 0.2126;
const LUMA_G: f32 = 0.7152;
const LUMA_B: f32 = 0.0722;

const SHADER_TEMPLATE: &str = r#"#version 300 es
precision highp float;

in vec2 v_texcoord;
uniform sampler2D tex;
out vec4 fragColor;

// Vogix monochromatic shader — auto-generated from theme palette
const vec3 themeColor = vec3({R}, {G}, {B});
const float intensity = {INTENSITY};
const float brightness = {BRIGHTNESS};
const float isDark = {IS_DARK};

// Theme functional colors (base08-0F) — preserved through tinting
{FUNCTIONAL_COLORS}

// Check if pixel is close to any functional color
float functionalMatch(vec3 c) {
    float closest = 1.0;
{FUNCTIONAL_DISTANCES}
    return 1.0 - smoothstep(0.0, 0.02, closest);
}

void main() {
    vec4 pixColor = texture(tex, v_texcoord);

    float luminance = dot(pixColor.rgb, vec3({LUMA_R}, {LUMA_G}, {LUMA_B}));

    // Dark: multiply — pulls toward black with theme hue
    vec3 darkMono = luminance * themeColor * brightness;
    // Light: screen blend — pulls toward white with theme hue
    vec3 lightMono = (1.0 - (1.0 - luminance) * (1.0 - themeColor)) * brightness;

    vec3 mono = mix(lightMono, darkMono, isDark);

    // Functional colors are preserved, everything else gets tinted
    float preserve = functionalMatch(pixColor.rgb);
    float tintStrength = intensity * (1.0 - preserve);
    vec3 result = mix(pixColor.rgb, mono, tintStrength);
    fragColor = vec4(result, pixColor.a);
}
"#;

/// Shader parameters with defaults.
#[derive(Debug, Clone)]
pub struct ShaderParams {
    /// Blend intensity between original and monochrome [0.0..1.0]
    pub intensity: f32,
    /// Output brightness multiplier [0.1..2.0]
    pub brightness: f32,
    /// Color saturation adjustment [0.0..2.0]
    pub saturation: f32,
}

impl Default for ShaderParams {
    fn default() -> Self {
        Self {
            intensity: 0.5,
            brightness: 1.0,
            saturation: 1.0,
        }
    }
}

/// Adjust saturation of a shader color.
/// < 1.0 desaturates toward gray, > 1.0 boosts vividness.
fn with_saturation(color: &ShaderColor, saturation: f32) -> ShaderColor {
    let gray = color.r * LUMA_R + color.g * LUMA_G + color.b * LUMA_B;
    ShaderColor::new(
        (gray + (color.r - gray) * saturation).clamp(0.0, 1.0),
        (gray + (color.g - gray) * saturation).clamp(0.0, 1.0),
        (gray + (color.b - gray) * saturation).clamp(0.0, 1.0),
    )
}

/// Generate GLSL shader source with embedded theme colors.
#[must_use]
pub fn generate_glsl(color: &ShaderColor, params: &ShaderParams, palette: &Palette) -> String {
    let color = with_saturation(color, params.saturation);

    // Functional (accent) colors from the ontology palette, by role, in slot
    // order — scheme-correct, deduplicated, byte-stable. Replaces the old
    // "try every naming convention" string shotgun.
    let mut func_consts = String::new();
    let mut func_dists = String::new();
    for (i, rgb) in crate::theme::palette::functional_colors(palette)
        .iter()
        .enumerate()
    {
        func_consts.push_str(&format!(
            "const vec3 func{} = vec3({:.4}, {:.4}, {:.4});\n",
            i,
            rgb.r as f32 / 255.0,
            rgb.g as f32 / 255.0,
            rgb.b as f32 / 255.0,
        ));
        func_dists.push_str(&format!(
            "    closest = min(closest, distance(c, func{}));\n",
            i
        ));
    }

    // Polarity from the ontology (base00 luminance). build_palette maps ansi16's
    // color00/background → Base00, so light ansi16 themes get the light (screen)
    // blend instead of silently defaulting to dark.
    let is_dark = {
        use pr4xis_domains::applied::hmi::theming::base16::Polarity;
        use pr4xis_domains::applied::hmi::theming::ontology;
        ontology::detect_polarity(palette) != Some(Polarity::Light)
    };

    SHADER_TEMPLATE
        .replace("{R}", &format!("{:.4}", color.r))
        .replace("{G}", &format!("{:.4}", color.g))
        .replace("{B}", &format!("{:.4}", color.b))
        .replace(
            "{INTENSITY}",
            &format!("{:.4}", params.intensity.clamp(0.0, 1.0)),
        )
        .replace(
            "{BRIGHTNESS}",
            &format!("{:.4}", params.brightness.clamp(0.1, 2.0)),
        )
        .replace("{IS_DARK}", if is_dark { "1.0" } else { "0.0" })
        .replace("{LUMA_R}", &format!("{:.4}", LUMA_R))
        .replace("{LUMA_G}", &format!("{:.4}", LUMA_G))
        .replace("{LUMA_B}", &format!("{:.4}", LUMA_B))
        .replace("{FUNCTIONAL_COLORS}", &func_consts)
        .replace("{FUNCTIONAL_DISTANCES}", &func_dists)
}

/// Return the directory for shader files at runtime.
/// Prefers `$XDG_RUNTIME_DIR/vogix/`, falls back to `/tmp/vogix/`.
pub fn shader_dir() -> Result<PathBuf> {
    let base = std::env::var("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp"));

    if !base.exists() {
        return Err(VogixError::NoRuntimeDir);
    }

    Ok(base.join("vogix"))
}

/// Write the shader to disk and return its path.
pub fn write_shader(
    color: &ShaderColor,
    params: &ShaderParams,
    palette: &Palette,
) -> Result<PathBuf> {
    let dir = shader_dir()?;
    fs::create_dir_all(&dir).map_err(|e| VogixError::ShaderWrite {
        path: dir.clone(),
        source: e,
    })?;

    let path = dir.join("monochromatic.glsl");
    let source = generate_glsl(color, params, palette);

    fs::write(&path, source).map_err(|e| VogixError::ShaderWrite {
        path: path.clone(),
        source: e,
    })?;

    log::info!("Wrote shader to {}", path.display());
    Ok(path)
}

/// Remove the shader file if it exists.
pub fn cleanup_shader() -> Result<()> {
    let dir = shader_dir()?;
    let path = dir.join("monochromatic.glsl");

    if path.exists() {
        fs::remove_file(&path).map_err(|e| VogixError::ShaderRemove {
            path: path.clone(),
            source: e,
        })?;
        log::debug!("Removed {}", path.display());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use pr4xis_domains::applied::hmi::theming::base16::ColorSlot;
    use pr4xis_domains::natural::colors::Rgb;

    fn green() -> ShaderColor {
        ShaderColor::new(0.0, 1.0, 0.0)
    }

    fn amber() -> ShaderColor {
        ShaderColor::new(1.0, 0.71, 0.0)
    }

    fn empty_palette() -> Palette {
        Palette::new()
    }

    /// Palette with the 8 base16 accents (base08..0F) populated.
    fn test_palette() -> Palette {
        let accents = [
            (ColorSlot::Base08, "#ff6b6b"),
            (ColorSlot::Base09, "#e89a4f"),
            (ColorSlot::Base0A, "#d4c44a"),
            (ColorSlot::Base0B, "#33ff66"),
            (ColorSlot::Base0C, "#66e5c0"),
            (ColorSlot::Base0D, "#5bb8e8"),
            (ColorSlot::Base0E, "#c484e8"),
            (ColorSlot::Base0F, "#7a8a55"),
        ];
        let mut p = Palette::new();
        for (slot, hex) in accents {
            p.insert(slot, Rgb::from_hex(hex).unwrap());
        }
        p
    }

    #[test]
    fn generate_contains_version() {
        let src = generate_glsl(&green(), &ShaderParams::default(), &empty_palette());
        assert!(src.starts_with("#version 300 es"));
    }

    #[test]
    fn generate_contains_theme_color() {
        let src = generate_glsl(&green(), &ShaderParams::default(), &empty_palette());
        assert!(src.contains("vec3(0.0000, 1.0000, 0.0000)"));
    }

    #[test]
    fn generate_amber_color() {
        let src = generate_glsl(&amber(), &ShaderParams::default(), &empty_palette());
        assert!(src.contains("vec3(1.0000, 0.7100, 0.0000)"));
    }

    #[test]
    fn generate_contains_intensity() {
        let params = ShaderParams {
            intensity: 0.8,
            ..Default::default()
        };
        let src = generate_glsl(&green(), &params, &empty_palette());
        assert!(src.contains("const float intensity = 0.8000;"));
    }

    #[test]
    fn generate_clamps_intensity() {
        let params = ShaderParams {
            intensity: 2.0,
            ..Default::default()
        };
        let src = generate_glsl(&green(), &params, &empty_palette());
        assert!(src.contains("const float intensity = 1.0000;"));
    }

    #[test]
    fn generate_brightness() {
        let params = ShaderParams {
            brightness: 0.5,
            ..Default::default()
        };
        let src = generate_glsl(&green(), &params, &empty_palette());
        assert!(src.contains("const float brightness = 0.5000;"));
    }

    #[test]
    fn generate_saturation_desaturate() {
        let params = ShaderParams {
            saturation: 0.0,
            ..Default::default()
        };
        let src = generate_glsl(&green(), &params, &empty_palette());
        assert!(src.contains("vec3(0.7152, 0.7152, 0.7152)"));
    }

    #[test]
    fn generate_has_valid_glsl_structure() {
        let src = generate_glsl(&green(), &ShaderParams::default(), &empty_palette());
        assert!(src.contains("void main()"));
        assert!(src.contains("fragColor ="));
        assert!(src.contains("texture(tex, v_texcoord)"));
        assert!(src.contains("luminance"));
        assert!(src.contains("functionalMatch"));
        assert!(src.contains("tintStrength"));
    }

    #[test]
    fn generate_embeds_functional_colors() {
        let src = generate_glsl(&green(), &ShaderParams::default(), &test_palette());
        assert!(
            src.contains("const vec3 func0"),
            "base08 should be embedded"
        );
        assert!(
            src.contains("const vec3 func7"),
            "base0F should be embedded"
        );
        assert!(
            src.contains("distance(c, func0)"),
            "distance check for base08"
        );
    }

    #[test]
    fn generate_no_functional_without_colors() {
        let src = generate_glsl(&green(), &ShaderParams::default(), &empty_palette());
        assert!(
            !src.contains("const vec3 func0"),
            "no func colors without palette"
        );
    }

    #[test]
    fn shader_dir_returns_path() {
        let dir = shader_dir().unwrap();
        assert!(dir.to_string_lossy().ends_with("/vogix"));
    }

    #[test]
    fn with_saturation_identity() {
        let c = ShaderColor::new(0.5, 0.3, 0.8);
        let s = with_saturation(&c, 1.0);
        assert!((s.r - c.r).abs() < 0.001);
        assert!((s.g - c.g).abs() < 0.001);
        assert!((s.b - c.b).abs() < 0.001);
    }

    #[test]
    fn with_saturation_zero_gives_gray() {
        let c = ShaderColor::new(0.0, 1.0, 0.0);
        let s = with_saturation(&c, 0.0);
        assert!((s.r - s.g).abs() < 0.001);
        assert!((s.g - s.b).abs() < 0.001);
    }

    #[test]
    fn with_saturation_clamps() {
        let c = ShaderColor::new(0.0, 1.0, 0.0);
        let s = with_saturation(&c, 2.0);
        assert!(s.r >= 0.0 && s.r <= 1.0);
        assert!(s.g >= 0.0 && s.g <= 1.0);
        assert!(s.b >= 0.0 && s.b <= 1.0);
    }

    fn dark_palette() -> Palette {
        let mut p = test_palette();
        p.insert(ColorSlot::Base00, Rgb::from_hex("#1e1e2e").unwrap()); // dark bg
        p
    }

    fn light_palette() -> Palette {
        let mut p = test_palette();
        p.insert(ColorSlot::Base00, Rgb::from_hex("#eff1f5").unwrap()); // light bg
        p
    }

    #[test]
    fn generate_dark_theme_uses_multiply() {
        let src = generate_glsl(&green(), &ShaderParams::default(), &dark_palette());
        assert!(src.contains("const float isDark = 1.0;"));
    }

    #[test]
    fn generate_light_theme_uses_screen() {
        let src = generate_glsl(&green(), &ShaderParams::default(), &light_palette());
        assert!(src.contains("const float isDark = 0.0;"));
    }

    #[test]
    fn generate_no_base00_defaults_dark() {
        let src = generate_glsl(&green(), &ShaderParams::default(), &empty_palette());
        assert!(src.contains("const float isDark = 1.0;"));
    }

    // Regression: ansi16 themes have no `base00` string key — build_palette maps
    // color00/background → Base00, so a LIGHT ansi16 theme uses the screen blend
    // instead of silently defaulting to dark (Copilot review, generator.rs:189).
    #[test]
    fn generate_ansi16_light_uses_screen() {
        use crate::scheme::Scheme;
        let mut c = std::collections::HashMap::new();
        c.insert("background".to_string(), "#eff1f5".to_string()); // light bg
        c.insert("color01".to_string(), "#d20f39".to_string()); // a red accent
        let palette = crate::theme::palette::build_palette(&c, Scheme::Ansi16);
        let src = generate_glsl(&green(), &ShaderParams::default(), &palette);
        assert!(
            src.contains("const float isDark = 0.0;"),
            "light ansi16 theme should use the screen (light) blend"
        );
        assert!(src.contains("const vec3 func0"), "color01 accent preserved");
    }

    #[test]
    fn generate_ansi16_dark_uses_multiply() {
        use crate::scheme::Scheme;
        let mut c = std::collections::HashMap::new();
        c.insert("background".to_string(), "#1e1e2e".to_string()); // dark bg
        let palette = crate::theme::palette::build_palette(&c, Scheme::Ansi16);
        let src = generate_glsl(&green(), &ShaderParams::default(), &palette);
        assert!(src.contains("const float isDark = 1.0;"));
    }
}

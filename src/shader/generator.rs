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

/// Get functional color keys from the praxis theming ontology.
///
/// Functional colors = Accent + BrightAccent roles across ALL scheme naming conventions.
/// The ontology defines which slots are functional — we map them to all known key formats:
/// - base16/base24: base08, base09, ..., base0F, base12, ..., base17
/// - vogix16: danger, success, warning, link, active, highlight, special, notice
/// - ansi16: color01, color02, ..., color15 (via ANSI index mapping)
///
/// These are preserved through the monochromatic tint so UI elements stay readable.
fn functional_color_keys() -> Vec<String> {
    use praxis::category::Entity;
    use praxis_domains::technology::theming::base16::{ColorSlot, SemanticRole};

    let mut keys = Vec::new();

    for slot in ColorSlot::variants() {
        if !matches!(slot.role(), SemanticRole::Accent | SemanticRole::BrightAccent) {
            continue;
        }

        // base16/base24 key name from ontology
        keys.push(slot.key().to_string());

        // ansi16 key name (via ANSI index)
        if let Some(idx) = slot.ansi_index() {
            keys.push(format!("color{:02}", idx));
        }
    }

    // vogix16 semantic names (these map to accent roles)
    keys.extend(
        [
            "danger", "success", "warning", "link", "active", "highlight", "special", "notice",
        ]
        .iter()
        .map(|s| s.to_string()),
    );

    keys
}

/// Generate GLSL shader source with embedded theme colors.
#[must_use]
pub fn generate_glsl(
    color: &ShaderColor,
    params: &ShaderParams,
    colors: &std::collections::HashMap<String, String>,
) -> String {
    let color = with_saturation(color, params.saturation);

    // Discover functional colors from praxis ontology (accent + bright accent)
    let functional_keys = functional_color_keys();
    let mut func_consts = String::new();
    let mut func_dists = String::new();
    let mut func_idx = 0;
    for key in &functional_keys {
        if let Some(hex) = colors.get(key.as_str())
            && let Some((r, g, b)) = super::color::hex_to_rgb(hex)
        {
            func_consts.push_str(&format!(
                "const vec3 func{} = vec3({:.4}, {:.4}, {:.4});\n",
                func_idx, r, g, b
            ));
            func_dists.push_str(&format!(
                "    closest = min(closest, distance(c, func{}));\n",
                func_idx
            ));
            func_idx += 1;
        }
    }

    // Detect polarity using praxis theming ontology
    // Build a palette from the hex colors, then ask the ontology for polarity
    let is_dark = {
        use praxis_domains::science::colors::Rgb;
        use praxis_domains::technology::theming::base16::{ColorSlot, Polarity};
        use praxis_domains::technology::theming::ontology;

        let mut palette = ontology::Palette::new();
        if let Some(hex) = colors.get("base00")
            && let Some(rgb) = Rgb::from_hex(hex)
        {
            palette.insert(ColorSlot::Base00, rgb);
        }
        ontology::detect_polarity(&palette) != Some(Polarity::Light)
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
    colors: &std::collections::HashMap<String, String>,
) -> Result<PathBuf> {
    let dir = shader_dir()?;
    fs::create_dir_all(&dir).map_err(|e| VogixError::ShaderWrite {
        path: dir.clone(),
        source: e,
    })?;

    let path = dir.join("monochromatic.glsl");
    let source = generate_glsl(color, params, colors);

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
    use std::collections::HashMap;

    fn green() -> ShaderColor {
        ShaderColor::new(0.0, 1.0, 0.0)
    }

    fn amber() -> ShaderColor {
        ShaderColor::new(1.0, 0.71, 0.0)
    }

    fn empty_colors() -> HashMap<String, String> {
        HashMap::new()
    }

    fn test_colors() -> HashMap<String, String> {
        let mut c = HashMap::new();
        c.insert("base08".into(), "#ff6b6b".into());
        c.insert("base09".into(), "#e89a4f".into());
        c.insert("base0A".into(), "#d4c44a".into());
        c.insert("base0B".into(), "#33ff66".into());
        c.insert("base0C".into(), "#66e5c0".into());
        c.insert("base0D".into(), "#5bb8e8".into());
        c.insert("base0E".into(), "#c484e8".into());
        c.insert("base0F".into(), "#7a8a55".into());
        c
    }

    #[test]
    fn generate_contains_version() {
        let src = generate_glsl(&green(), &ShaderParams::default(), &empty_colors());
        assert!(src.starts_with("#version 300 es"));
    }

    #[test]
    fn generate_contains_theme_color() {
        let src = generate_glsl(&green(), &ShaderParams::default(), &empty_colors());
        assert!(src.contains("vec3(0.0000, 1.0000, 0.0000)"));
    }

    #[test]
    fn generate_amber_color() {
        let src = generate_glsl(&amber(), &ShaderParams::default(), &empty_colors());
        assert!(src.contains("vec3(1.0000, 0.7100, 0.0000)"));
    }

    #[test]
    fn generate_contains_intensity() {
        let params = ShaderParams {
            intensity: 0.8,
            ..Default::default()
        };
        let src = generate_glsl(&green(), &params, &empty_colors());
        assert!(src.contains("const float intensity = 0.8000;"));
    }

    #[test]
    fn generate_clamps_intensity() {
        let params = ShaderParams {
            intensity: 2.0,
            ..Default::default()
        };
        let src = generate_glsl(&green(), &params, &empty_colors());
        assert!(src.contains("const float intensity = 1.0000;"));
    }

    #[test]
    fn generate_brightness() {
        let params = ShaderParams {
            brightness: 0.5,
            ..Default::default()
        };
        let src = generate_glsl(&green(), &params, &empty_colors());
        assert!(src.contains("const float brightness = 0.5000;"));
    }

    #[test]
    fn generate_saturation_desaturate() {
        let params = ShaderParams {
            saturation: 0.0,
            ..Default::default()
        };
        let src = generate_glsl(&green(), &params, &empty_colors());
        assert!(src.contains("vec3(0.7152, 0.7152, 0.7152)"));
    }

    #[test]
    fn generate_has_valid_glsl_structure() {
        let src = generate_glsl(&green(), &ShaderParams::default(), &empty_colors());
        assert!(src.contains("void main()"));
        assert!(src.contains("fragColor ="));
        assert!(src.contains("texture(tex, v_texcoord)"));
        assert!(src.contains("luminance"));
        assert!(src.contains("functionalMatch"));
        assert!(src.contains("tintStrength"));
    }

    #[test]
    fn generate_embeds_functional_colors() {
        let src = generate_glsl(&green(), &ShaderParams::default(), &test_colors());
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
        let src = generate_glsl(&green(), &ShaderParams::default(), &empty_colors());
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

    fn dark_colors() -> HashMap<String, String> {
        let mut c = test_colors();
        c.insert("base00".into(), "#1e1e2e".into()); // dark bg
        c
    }

    fn light_colors() -> HashMap<String, String> {
        let mut c = test_colors();
        c.insert("base00".into(), "#eff1f5".into()); // light bg
        c
    }

    #[test]
    fn generate_dark_theme_uses_multiply() {
        let src = generate_glsl(&green(), &ShaderParams::default(), &dark_colors());
        assert!(src.contains("const float isDark = 1.0;"));
    }

    #[test]
    fn generate_light_theme_uses_screen() {
        let src = generate_glsl(&green(), &ShaderParams::default(), &light_colors());
        assert!(src.contains("const float isDark = 0.0;"));
    }

    #[test]
    fn generate_no_base00_defaults_dark() {
        let src = generate_glsl(&green(), &ShaderParams::default(), &empty_colors());
        assert!(src.contains("const float isDark = 1.0;"));
    }
}

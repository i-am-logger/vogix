//! Shader management — auto-apply on theme change + manual toggle.
//!
//! The shader auto-applies when `[shader] enabled = true` in config.
//! Manual toggle via `vogix shader on/off/toggle` for quick switching.

use crate::config::Config;
use crate::errors::Result;
use crate::shader;
use crate::shader::generator::ShaderParams;
use crate::state::State;

/// Auto-apply or clear shader after a theme change/refresh.
///
/// Called from theme_change and refresh handlers. Reads shader config
/// from the system config — if enabled, generates and applies the shader
/// from the current theme's colors. If disabled, clears any active shader.
pub fn maybe_apply_shader(config: &Config, state: &State) -> Result<()> {
    let shader_config = match &config.shader {
        Some(sc) if sc.enabled => sc,
        _ => {
            // Shader not configured or disabled — clear if previously active
            let _ = shader::disable();
            return Ok(());
        }
    };

    let params = ShaderParams {
        intensity: shader_config.intensity,
        brightness: shader_config.brightness,
        saturation: shader_config.saturation,
    };

    let colors = load_current_theme_colors(config, state)?;
    shader::apply_from_colors(&colors, &params)?;

    log::debug!(
        "Shader applied for {}-{}",
        state.current_theme,
        state.current_variant
    );
    Ok(())
}

/// Load theme colors for the current theme/variant/scheme from source files.
fn load_current_theme_colors(
    config: &Config,
    state: &State,
) -> Result<std::collections::HashMap<String, String>> {
    let theme_sources = config.theme_sources.as_ref().ok_or_else(|| {
        crate::errors::VogixError::Config(
            "theme_sources not configured — shader needs theme color files".to_string(),
        )
    })?;

    let variant_path = crate::cache::paths::theme_variant_path(
        theme_sources,
        &state.current_scheme,
        &state.current_theme,
        &state.current_variant,
    );

    crate::theme::load_theme_colors(&variant_path, state.current_scheme)
}

/// Turn shader on — apply current theme's monochromatic tint.
/// Optional overrides for intensity/brightness/saturation.
pub fn handle_shader_on(
    intensity: Option<f32>,
    brightness: Option<f32>,
    saturation: Option<f32>,
) -> Result<()> {
    let config = Config::load()?;
    let state = State::load()?;

    let shader_config = config.shader.as_ref();
    let base = match shader_config {
        Some(sc) => ShaderParams {
            intensity: sc.intensity,
            brightness: sc.brightness,
            saturation: sc.saturation,
        },
        None => ShaderParams::default(),
    };

    let params = ShaderParams {
        intensity: intensity.unwrap_or(base.intensity),
        brightness: brightness.unwrap_or(base.brightness),
        saturation: saturation.unwrap_or(base.saturation),
    };

    let colors = load_current_theme_colors(&config, &state)?;
    shader::apply_from_colors(&colors, &params)?;

    log::info!("Shader on");
    Ok(())
}

/// Turn shader off.
pub fn handle_shader_off() -> Result<()> {
    shader::disable()?;
    log::info!("Shader off");
    Ok(())
}

/// Toggle shader — check if active, flip it.
pub fn handle_shader_toggle() -> Result<()> {
    // Check if shader is currently active by reading hyprctl
    let output = std::process::Command::new("hyprctl")
        .args(["getoption", "decoration:screen_shader", "-j"])
        .output();

    let is_active = match output {
        Ok(o) if o.status.success() => {
            let stdout = String::from_utf8_lossy(&o.stdout);
            // If the shader path is set (not empty), it's active
            stdout.contains("/vogix/") && !stdout.contains("[[EMPTY]]")
        }
        _ => false,
    };

    if is_active {
        handle_shader_off()
    } else {
        handle_shader_on(None, None, None)
    }
}

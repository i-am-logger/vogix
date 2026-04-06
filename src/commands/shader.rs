//! Automatic shader application on theme change.
//!
//! The shader is a property of every theme — auto-generated from
//! the base00-07 palette hue. No CLI needed; it's applied automatically
//! when `[shader] enabled = true` in config (set by Nix module).

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

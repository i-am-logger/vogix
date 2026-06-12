//! Shader management — auto-apply on theme change + manual toggle.
//!
//! The shader auto-applies based on ShaderState (On/Off/Auto).
//! Manual toggle via `vogix shader on/off/toggle` for quick switching.

use crate::config::Config;
use crate::errors::Result;
use crate::shader;
use crate::shader::generator::ShaderParams;
use crate::state::{ShaderState, State};

/// Auto-apply or clear shader after a theme change/refresh.
///
/// Called from theme_change and refresh handlers. Uses ShaderState enum:
/// - On: apply with stored params
/// - Off: clear shader
/// - Auto: follow config default
pub fn maybe_apply_shader(config: &Config, state: &State) -> Result<()> {
    let should_enable = match &state.shader {
        ShaderState::On { .. } => true,
        ShaderState::Off => false,
        ShaderState::Auto => config.shader.as_ref().is_some_and(|sc| sc.enabled),
    };

    if !should_enable {
        let _ = shader::disable();
        return Ok(());
    }

    let params = resolve_shader_params(config, state);
    let colors = load_current_theme_colors(config, state)?;
    shader::apply_from_colors(&colors, &params)?;

    log::debug!(
        "Shader applied for {}-{}",
        state.current_theme,
        state.current_variant
    );
    Ok(())
}

/// Re-apply the shader after a Hyprland config reload WITHOUT regenerating it.
///
/// A config reload resets `decoration:screen_shader`, but the generated GLSL is
/// still on disk. Re-point hyprctl at the existing file rather than regenerating
/// from theme colors — regeneration needs `theme_sources`, which isn't present in
/// every config (notably the daemon's `Config::load()`). Only if the file is
/// missing do we fall back to a full regenerate.
pub fn reapply_existing_shader(config: &Config, state: &State) -> Result<()> {
    let should_enable = match &state.shader {
        ShaderState::On { .. } => true,
        ShaderState::Off => false,
        ShaderState::Auto => config.shader.as_ref().is_some_and(|sc| sc.enabled),
    };

    if !should_enable {
        let _ = shader::disable();
        return Ok(());
    }

    let path = shader::generator::shader_dir()?.join("monochromatic.glsl");
    if path.exists() {
        shader::hyprctl::set_shader(&path)?;
        log::debug!("Re-applied existing shader: {}", path.display());
        Ok(())
    } else {
        // No pre-generated shader on disk — fall back to a full (re)generate.
        maybe_apply_shader(config, state)
    }
}

/// Load theme colors for the current theme/variant/scheme from source files.
pub fn load_current_theme_colors(
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

/// Show shader status and current parameters.
pub fn handle_shader_status() -> Result<()> {
    let config = Config::load()?;
    let state = State::load()?;

    let params = resolve_shader_params(&config, &state);
    let status = match &state.shader {
        ShaderState::On { .. } => "on",
        ShaderState::Off => "off",
        ShaderState::Auto => {
            if config.shader.as_ref().is_some_and(|sc| sc.enabled) {
                "on (auto)"
            } else {
                "off (auto)"
            }
        }
    };

    println!("Shader: {}", status);
    println!("Intensity:  {:.2}", params.intensity);
    println!("Brightness: {:.2}", params.brightness);
    println!("Saturation: {:.2}", params.saturation);

    Ok(())
}

/// Resolve shader params: ShaderState params > config defaults > ShaderParams::default()
pub fn resolve_shader_params(config: &Config, state: &State) -> ShaderParams {
    let base = match &config.shader {
        Some(sc) => ShaderParams {
            intensity: sc.intensity,
            brightness: sc.brightness,
            saturation: sc.saturation,
        },
        None => ShaderParams::default(),
    };

    match &state.shader {
        ShaderState::On {
            intensity,
            brightness,
            saturation,
        } => ShaderParams {
            intensity: *intensity,
            brightness: *brightness,
            saturation: *saturation,
        },
        _ => base,
    }
}

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

/// Turn shader on — apply current theme's monochromatic tint.
pub fn handle_shader_on(
    intensity: Option<f32>,
    brightness: Option<f32>,
    saturation: Option<f32>,
) -> Result<()> {
    let config = Config::load()?;
    let mut state = State::load()?;

    let (base_i, base_b, base_s) = state.shader.params().unwrap_or((0.5, 1.0, 1.0));

    state.shader = ShaderState::On {
        intensity: intensity.unwrap_or(base_i),
        brightness: brightness.unwrap_or(base_b),
        saturation: saturation.unwrap_or(base_s),
    };

    let params = resolve_shader_params(&config, &state);
    let colors = load_current_theme_colors(&config, &state)?;
    shader::apply_from_colors(&colors, &params)?;

    state.save()?;
    log::info!("Shader on");
    Ok(())
}

/// Turn shader off.
pub fn handle_shader_off() -> Result<()> {
    shader::disable()?;

    let mut state = State::load()?;
    state.shader = ShaderState::Off;
    state.save()?;

    log::info!("Shader off");
    Ok(())
}

/// Toggle shader — check ShaderState, flip it.
pub fn handle_shader_toggle() -> Result<()> {
    let state = State::load()?;

    match &state.shader {
        ShaderState::On { .. } => handle_shader_off(),
        ShaderState::Off => handle_shader_on(None, None, None),
        ShaderState::Auto => {
            let config = Config::load()?;
            let config_enabled = config.shader.as_ref().is_some_and(|sc| sc.enabled);
            if config_enabled {
                handle_shader_off()
            } else {
                handle_shader_on(None, None, None)
            }
        }
    }
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

/// Set a single shader parameter, persist it, and re-apply if shader is on.
pub fn handle_shader_param(param: &str, value: f32) -> Result<()> {
    let mut state = State::load()?;

    if let ShaderState::On {
        intensity,
        brightness,
        saturation,
    } = &state.shader
    {
        let (mut i, mut b, mut s) = (*intensity, *brightness, *saturation);
        match param {
            "intensity" => i = value.clamp(0.0, 1.0),
            "brightness" => b = value.clamp(0.1, 2.0),
            "saturation" => s = value.clamp(0.0, 2.0),
            _ => unreachable!(),
        }
        state.shader = ShaderState::On {
            intensity: i,
            brightness: b,
            saturation: s,
        };
        state.save()?;

        let config = Config::load()?;
        let params = resolve_shader_params(&config, &state);
        let colors = load_current_theme_colors(&config, &state)?;
        shader::apply_from_colors(&colors, &params)?;
    } else {
        log::warn!("Shader is off — turn it on first");
    }

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

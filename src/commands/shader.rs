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
/// Called from theme_change and refresh handlers. Uses persisted state
/// (`state.shader_enabled`) to decide whether to apply. Falls back to
/// config if state hasn't been set yet (first boot with shader configured).
pub fn maybe_apply_shader(config: &Config, state: &State) -> Result<()> {
    // None = follow config, Some(true/false) = user override
    let config_enabled = config.shader.as_ref().is_some_and(|sc| sc.enabled);
    let should_enable = state.shader_enabled.unwrap_or(config_enabled);

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
    let mut state = State::load()?;

    // Apply any explicit overrides to state
    if intensity.is_some() {
        state.shader_intensity = intensity;
    }
    if brightness.is_some() {
        state.shader_brightness = brightness;
    }
    if saturation.is_some() {
        state.shader_saturation = saturation;
    }

    let params = resolve_shader_params(&config, &state);
    let colors = load_current_theme_colors(&config, &state)?;
    shader::apply_from_colors(&colors, &params)?;

    state.shader_enabled = Some(true);
    state.save()?;

    log::info!("Shader on");
    Ok(())
}

/// Turn shader off.
pub fn handle_shader_off() -> Result<()> {
    shader::disable()?;

    let mut state = State::load()?;
    state.shader_enabled = Some(false);
    state.save()?;

    log::info!("Shader off");
    Ok(())
}

/// Toggle shader — check persisted state, flip it.
pub fn handle_shader_toggle() -> Result<()> {
    let state = State::load()?;

    let config = Config::load()?;
    let config_enabled = config.shader.as_ref().is_some_and(|sc| sc.enabled);
    if state.shader_enabled.unwrap_or(config_enabled) {
        handle_shader_off()
    } else {
        handle_shader_on(None, None, None)
    }
}

/// Show shader status and current parameters.
pub fn handle_shader_status() -> Result<()> {
    let config = Config::load()?;
    let state = State::load()?;

    let params = resolve_shader_params(&config, &state);

    println!(
        "Shader: {}",
        if state.shader_enabled.unwrap_or(false) {
            "on"
        } else {
            "off"
        }
    );
    println!("Intensity:  {:.2}", params.intensity);
    println!("Brightness: {:.2}", params.brightness);
    println!("Saturation: {:.2}", params.saturation);

    Ok(())
}

/// Set a single shader parameter, persist it, and re-apply if shader is on.
pub fn handle_shader_param(param: &str, value: f32) -> Result<()> {
    let mut state = State::load()?;

    match param {
        "intensity" => state.shader_intensity = Some(value.clamp(0.0, 1.0)),
        "brightness" => state.shader_brightness = Some(value.clamp(0.1, 2.0)),
        "saturation" => state.shader_saturation = Some(value.clamp(0.0, 2.0)),
        _ => unreachable!(),
    }

    state.save()?;

    if state.shader_enabled.unwrap_or(false) {
        let config = Config::load()?;
        let params = resolve_shader_params(&config, &state);
        let colors = load_current_theme_colors(&config, &state)?;
        shader::apply_from_colors(&colors, &params)?;
    }

    Ok(())
}

/// Resolve shader params: state overrides > config defaults > ShaderParams::default()
fn resolve_shader_params(config: &Config, state: &State) -> ShaderParams {
    let base = match &config.shader {
        Some(sc) => ShaderParams {
            intensity: sc.intensity,
            brightness: sc.brightness,
            saturation: sc.saturation,
        },
        None => ShaderParams::default(),
    };

    ShaderParams {
        intensity: state.shader_intensity.unwrap_or(base.intensity),
        brightness: state.shader_brightness.unwrap_or(base.brightness),
        saturation: state.shader_saturation.unwrap_or(base.saturation),
    }
}

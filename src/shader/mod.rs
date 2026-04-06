//! Monochromatic screen shader module.
//!
//! Provides:
//! - Automatic shader color extraction from theme base00-07 palette
//! - GLSL shader generation for Hyprland's screen_shader
//! - Shader application and cleanup via hyprctl IPC
//!
//! The shader is a *property* of every vogix theme — auto-generated from
//! the dominant hue of the monochromatic scale. No manual color selection needed.

pub mod color;
pub mod generator;
pub mod hyprctl;

use crate::errors::Result;
use color::extract_shader_color;
use generator::ShaderParams;
use std::collections::HashMap;
use std::path::PathBuf;

/// Generate, write, and apply a monochromatic shader from theme colors.
///
/// Extracts the dominant hue from base00-07, generates a GLSL shader,
/// writes it to the runtime directory, and applies it via hyprctl.
pub fn apply_from_colors(
    colors: &HashMap<String, String>,
    params: &ShaderParams,
) -> Result<PathBuf> {
    hyprctl::check_environment()?;
    let shader_color = extract_shader_color(colors);
    let path = generator::write_shader(&shader_color, params, colors)?;
    hyprctl::set_shader(&path)?;
    Ok(path)
}

/// Clear the active screen shader and remove the shader file.
pub fn disable() -> Result<()> {
    hyprctl::check_environment()?;
    hyprctl::clear_shader()?;
    generator::cleanup_shader()?;
    Ok(())
}

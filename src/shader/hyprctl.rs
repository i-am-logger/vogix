//! Hyprland IPC for shader management.
//!
//! Applies and clears the `decoration:screen_shader` option over Hyprland's
//! control socket ([`crate::input::hypr::Hypr`]), which speaks the dialect of
//! whichever config engine the compositor runs — `keyword` under hyprlang,
//! `eval hl.config({…})` under the Lua engine (where `keyword` is rejected).
//! The `[[EMPTY]]` clear sentinel below is hyprlang's; the socket layer maps
//! it to `""` for Lua, which reads back as a cleanly empty option.

use crate::errors::{Result, VogixError};
use crate::input::hypr::Hypr;
use std::path::Path;

/// Check that Hyprland is running.
pub fn check_environment() -> Result<()> {
    if std::env::var("HYPRLAND_INSTANCE_SIGNATURE").is_err() {
        return Err(VogixError::HyprlandNotRunning);
    }
    Ok(())
}

/// Apply a screen shader from the given file path.
pub fn set_shader(shader_path: &Path) -> Result<()> {
    log::info!("Applying shader: {}", shader_path.display());
    set_screen_shader(&shader_path.to_string_lossy())
}

/// Clear the active screen shader.
pub fn clear_shader() -> Result<()> {
    log::info!("Clearing screen shader");
    set_screen_shader("[[EMPTY]]")
}

fn set_screen_shader(value: &str) -> Result<()> {
    let hypr = Hypr::discover().ok_or(VogixError::HyprlandNotRunning)?;
    hypr.set_keyword("decoration:screen_shader", value)
        .map_err(|e| VogixError::HyprctlFailed {
            code: -1,
            detail: e.to_string(),
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    #[serial]
    fn check_environment_no_signature() {
        let original = std::env::var("HYPRLAND_INSTANCE_SIGNATURE").ok();
        unsafe { std::env::remove_var("HYPRLAND_INSTANCE_SIGNATURE") };

        let err = check_environment().unwrap_err();
        assert!(matches!(err, VogixError::HyprlandNotRunning));

        if let Some(val) = original {
            unsafe { std::env::set_var("HYPRLAND_INSTANCE_SIGNATURE", val) };
        }
    }

    #[test]
    #[serial]
    fn check_environment_with_signature() {
        let original = std::env::var("HYPRLAND_INSTANCE_SIGNATURE").ok();
        unsafe { std::env::set_var("HYPRLAND_INSTANCE_SIGNATURE", "test_instance") };

        assert!(check_environment().is_ok());

        match original {
            Some(val) => unsafe { std::env::set_var("HYPRLAND_INSTANCE_SIGNATURE", val) },
            None => unsafe { std::env::remove_var("HYPRLAND_INSTANCE_SIGNATURE") },
        }
    }
}

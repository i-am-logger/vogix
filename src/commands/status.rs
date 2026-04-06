//! Status command - show current theme state.

use crate::errors::Result;
use crate::state::State;

/// Handle the `status` command - display current theme/variant/scheme
pub fn handle_status() -> Result<()> {
    let state = State::load()?;
    state.save()?;

    println!("scheme:  {}", state.current_scheme);
    println!("theme:   {}", state.current_theme);
    println!("variant: {}", state.current_variant);

    if let Some(ref last_applied) = state.last_applied {
        println!("applied: {}", last_applied);
    }

    // Check shader status via hyprctl
    let shader_active = std::process::Command::new("hyprctl")
        .args(["getoption", "decoration:screen_shader", "-j"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| {
            let s = String::from_utf8_lossy(&o.stdout);
            s.contains("/vogix/") && !s.contains("[[EMPTY]]")
        })
        .unwrap_or(false);

    println!("shader:  {}", if shader_active { "on" } else { "off" });

    Ok(())
}

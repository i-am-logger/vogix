//! Status command - show current theme state.

use crate::errors::Result;
use crate::state::State;

/// Handle the `status` command - display current theme/variant/scheme
pub fn handle_status() -> Result<()> {
    let state = State::load()?;
    state.save()?;

    println!(
        "scheme:  {} ({} slots)",
        state.current_scheme,
        state.current_scheme.slot_count()
    );
    println!("theme:   {}", state.current_theme);
    println!("variant: {}", state.current_variant);
    println!("mode:    {}", state.current_mode);

    // Which config engine the compositor runs — the wire dialect vogix speaks
    // (keyword/legacy dispatch vs eval/hl.dsp) follows it, so surfacing it
    // here makes a mismatch diagnosable at a glance during the Lua migration.
    if let Some(hypr) = crate::input::hypr::Hypr::discover() {
        let engine = match hypr.provider() {
            crate::input::hypr::ConfigProvider::Hyprlang => "hyprlang",
            crate::input::hypr::ConfigProvider::Lua => "lua",
        };
        println!("compositor config engine: {engine}");
    }

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

//! `vogix greeter` — the SDDM greeter's runtime follow.
//!
//! The greeter is themed at BUILD time from the machine owner's palette
//! (vogix.machine.owner; theme.conf); `greeter sync` layers the runtime on top: it COPIES the
//! live theme.json and the current background reference into
//! /var/lib/vogix/greeter (group-writable, tmpfiles from vogix's NixOS
//! module), which the greeter QML prefers over its build-time palette.
//! Copies, never links — the greeter runs before user homes are mounted,
//! and store paths swap on every switch.

use crate::cli::GreeterCommands;
use crate::config::Config;
use crate::errors::{Result, VogixError};
use crate::fsutil;
use std::path::Path;

const DROP_ZONE: &str = "/var/lib/vogix/greeter";

pub fn handle_greeter(command: &GreeterCommands) -> Result<()> {
    match command {
        GreeterCommands::Sync => sync(),
    }
}

fn sync() -> Result<()> {
    let dest = Path::new(DROP_ZONE);
    if !dest.is_dir() {
        return Err(VogixError::Config(format!(
            "{DROP_ZONE} does not exist — enable vogix.greeter.enable on the host \
             (it creates the group-writable drop zone)"
        )));
    }

    let state = Config::state_dir();
    let theme_src = state.join("current-theme/vogix-desktop/theme.json");
    let theme = std::fs::read(&theme_src)
        .map_err(|e| VogixError::Config(format!("cannot read {}: {e}", theme_src.display())))?;
    write_group_writable(&dest.join("theme.json"), &theme)?;

    // Background: the per-user override wins, else the theme set's first
    // entry. A store path is fine — the greeter can read /nix/store.
    if let Some(path) = current_background(&state) {
        let payload = serde_json::json!({ "path": path }).to_string();
        write_group_writable(&dest.join("background.json"), payload.as_bytes())?;
    }

    println!("greeter: synced {}", dest.display());
    Ok(())
}

/// Write into the shared drop zone with the symlink-proof atomic writer. The
/// zone is group-writable (any vogix user syncs, setgid directory), so the
/// file is left group-writable (0664) for the next user's sync to replace.
fn write_group_writable(dest: &Path, contents: &[u8]) -> Result<()> {
    fsutil::write_atomic(dest, contents, 0o664)
        .map(|_| ())
        .map_err(|e| VogixError::Config(format!("cannot write {}: {e}", dest.display())))
}

fn current_background(state: &Path) -> Option<String> {
    // Per-user override (the wallpaper layer's own state file).
    if let Ok(raw) = std::fs::read_to_string(state.join("desktop/background.json"))
        && let Ok(doc) = serde_json::from_str::<serde_json::Value>(&raw)
        && let Some(p) = doc.get("override").and_then(|v| v.as_str())
        && !p.is_empty()
    {
        return Some(p.to_string());
    }
    // The theme's own set, through the current-theme chain.
    let raw =
        std::fs::read_to_string(state.join("current-theme/vogix-desktop/backgrounds.json")).ok()?;
    let doc = serde_json::from_str::<serde_json::Value>(&raw).ok()?;
    doc.get("backgrounds")?
        .as_array()?
        .first()?
        .get("path")?
        .as_str()
        .map(str::to_string)
}

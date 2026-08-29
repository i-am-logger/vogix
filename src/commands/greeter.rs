//! `vogix greeter` — the SDDM greeter's runtime follow.
//!
//! The greeter is themed at BUILD time from the first vogix user's palette
//! (theme.conf); `greeter sync` layers the runtime on top: it COPIES the
//! live theme.json and the current background reference into
//! /var/lib/vogix/greeter (group-writable, tmpfiles from vogix's NixOS
//! module), which the greeter QML prefers over its build-time palette.
//! Copies, never links — the greeter runs before user homes are mounted,
//! and store paths swap on every switch.

use crate::cli::GreeterCommands;
use crate::config::Config;
use crate::errors::{Result, VogixError};
use std::os::unix::fs::PermissionsExt;
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

/// Atomic, symlink-proof write into the shared drop zone: the zone is
/// group-writable (any vogix user syncs), so a plain write/copy at the
/// destination could FOLLOW a pre-planted symlink and clobber an arbitrary
/// file. Create a fresh temp file (O_EXCL — never follows anything) and
/// rename() it over the target: rename replaces a symlink itself rather
/// than dereferencing it, and readers see old-or-new, never a torn file.
fn write_group_writable(dest: &Path, contents: &[u8]) -> Result<()> {
    use std::io::Write;

    let dir = dest
        .parent()
        .ok_or_else(|| VogixError::Config(format!("{} has no parent directory", dest.display())))?;
    let tmp = dir.join(format!(
        ".{}.tmp.{}",
        dest.file_name().and_then(|n| n.to_str()).unwrap_or("sync"),
        std::process::id()
    ));
    let err =
        |e: std::io::Error| VogixError::Config(format!("cannot write {}: {e}", dest.display()));

    let mut f = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&tmp)
        .map_err(err)?;
    let result = f
        .write_all(contents)
        .and_then(|()| {
            // The zone is shared by every vogix user (setgid dir); leave the
            // file group-writable so the NEXT user's sync can replace it.
            f.set_permissions(std::fs::Permissions::from_mode(0o664))
        })
        .and_then(|()| f.sync_all())
        .map_err(err)
        .and_then(|()| std::fs::rename(&tmp, dest).map_err(err));
    if result.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    result
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

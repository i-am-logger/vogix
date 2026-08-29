//! `vogix desktop` — desktop-shell verbs.
//!
//! The verbs are the CONTRACT between vogix and the shell: keybindings,
//! config.toml reload entries and consumer modules only ever call these. The
//! TRANSPORT behind them is an implementation detail — v1 relays to the
//! quickshell instance (`qs -c vogix ipc …`), v2 will speak the Rust
//! vogix-desktop's own socket — so swapping the shell renderer never touches
//! a caller.

use crate::cli::DesktopCommands;
use crate::errors::Result;
use log::debug;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

pub fn handle_desktop(command: &DesktopCommands) -> Result<()> {
    match command {
        DesktopCommands::Reload => reload(),
    }
}

/// Ask the running shell to re-read `theme.json` (and, once it exists,
/// `desktop.json`). The store-symlink swap a theme switch performs is
/// invisible to Qt's file watcher, so the reload is an explicit verb wired as
/// the app's `reload_command`.
///
/// No shell instance — a TTY session, tests, the shell not enabled — is
/// SUCCESS by design: this runs on every theme switch for every desktop
/// user, and a missing shell must never fail the switch. The wait is bounded
/// so a wedged shell cannot hang the switch either.
fn reload() -> Result<()> {
    let Ok(mut child) = Command::new("qs")
        .args(["-c", "vogix", "ipc", "call", "theme", "reload"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    else {
        debug!("desktop reload: quickshell (qs) not present — nothing to reload");
        return Ok(());
    };

    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                debug!("desktop reload: qs ipc exited with {status}");
                return Ok(());
            }
            Ok(None) if Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(25));
            }
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                debug!("desktop reload: qs ipc timed out — no responsive shell instance");
                return Ok(());
            }
        }
    }
}

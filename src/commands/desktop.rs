//! `vogix desktop` — desktop-shell verbs.
//!
//! The verbs are the CONTRACT between vogix and the shell: keybindings,
//! config.toml reload entries and consumer modules only ever call these. The
//! TRANSPORT behind them is an implementation detail — v1 relays to the
//! quickshell instance (`qs -c vogix ipc …`), v2 will speak the Rust
//! vogix-desktop's own socket — so swapping the shell renderer never touches
//! a caller.

use crate::cli::{BarCommands, DesktopCommands};
use crate::errors::Result;
use log::debug;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

pub fn handle_desktop(command: &DesktopCommands) -> Result<()> {
    match command {
        DesktopCommands::Reload => reload(),
        DesktopCommands::Status => status(),
        DesktopCommands::Bar { command } => bar(command),
    }
}

/// Run one `qs ipc` call against the vogix shell instance, bounded. Returns
/// the trimmed stdout, or None when no responsive instance exists. qs
/// conflates its failures onto stdout with exit 0 ("Target not found.", "No
/// running instances…", "Not ready to accept queries yet"), so those are
/// normalized to None here rather than surfacing as shell replies.
fn qs_ipc(args: &[&str]) -> Option<String> {
    let mut child = Command::new("qs")
        .args(["-c", "vogix", "ipc", "call"])
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;

    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(25));
            }
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
        }
    }
    let out = child.wait_with_output().ok()?;
    let reply = String::from_utf8_lossy(&out.stdout).trim().to_string();
    let failure = !out.status.success()
        || reply.starts_with("Target not found")
        || reply.starts_with("Function not found")
        || reply.starts_with("No running instances")
        || reply.starts_with("Not ready to accept queries");
    if failure { None } else { Some(reply) }
}

/// Whether a shell instance is running, and the bar state if so.
fn status() -> Result<()> {
    match qs_ipc(&["bar", "status"]) {
        Some(bar_state) => println!("shell: running\nbar: {bar_state}"),
        None => println!("shell: not running"),
    }
    Ok(())
}

fn bar(command: &BarCommands) -> Result<()> {
    let verb = match command {
        BarCommands::Show => "show",
        BarCommands::Hide => "hide",
        BarCommands::Toggle => "toggle",
    };
    if qs_ipc(&["bar", verb]).is_none() {
        debug!("desktop bar {verb}: no responsive shell instance");
    }
    Ok(())
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

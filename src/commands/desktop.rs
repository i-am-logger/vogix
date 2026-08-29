//! `vogix desktop` — desktop-shell verbs.
//!
//! The verbs are the CONTRACT between vogix and the shell: keybindings,
//! config.toml reload entries and consumer modules only ever call these. The
//! TRANSPORT behind them is an implementation detail — v1 relays to the
//! quickshell instance (`qs -c vogix ipc …`), v2 will speak the Rust
//! vogix-desktop's own socket — so swapping the shell renderer never touches
//! a caller.

use crate::cli::{BarCommands, DesktopCommands, DndCommands, LockCommands, NotifyCommands};
use crate::errors::{Result, VogixError};
use log::debug;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

pub fn handle_desktop(command: &DesktopCommands) -> Result<()> {
    match command {
        DesktopCommands::Reload => reload(),
        DesktopCommands::Status => status(),
        DesktopCommands::Bar { command } => bar(command),
        DesktopCommands::Notify { command } => notify(command),
        DesktopCommands::Lock {
            wait_secure,
            command,
        } => match command {
            Some(LockCommands::Status) => lock_status(),
            None => lock(*wait_secure),
        },
        DesktopCommands::Restart => restart(),
        DesktopCommands::Osd {
            kind,
            value,
            muted,
            message,
        } => osd(kind, *value, *muted, message.as_deref()),
    }
}

/// Engage the session lock. Unlike every other desktop verb, failure here is
/// LOUD: a `vogix desktop lock` (or $LOCKER, or the sleep hook) that quietly
/// does nothing leaves an unattended, unlocked machine.
fn lock(wait_secure: Option<f64>) -> Result<()> {
    let reply = qs_ipc(&["lock", "lock"]).ok_or_else(|| {
        VogixError::Config("cannot lock: no responsive vogix shell instance".to_string())
    })?;
    if reply.starts_with("refused") {
        return Err(VogixError::Config(format!("cannot lock: {reply}")));
    }
    if let Some(secs) = wait_secure {
        let deadline = Instant::now() + Duration::from_secs_f64(secs);
        loop {
            match qs_ipc(&["lock", "status"]).as_deref() {
                Some("secure") => break,
                _ if Instant::now() >= deadline => {
                    return Err(VogixError::Config(format!(
                        "lock engaged but not SECURE within {secs}s — an output may be uncovered"
                    )));
                }
                _ => std::thread::sleep(Duration::from_millis(50)),
            }
        }
    }
    Ok(())
}

fn lock_status() -> Result<()> {
    match qs_ipc(&["lock", "status"]) {
        Some(state) => println!("{state}"),
        None => println!("unlocked (no shell instance)"),
    }
    Ok(())
}

/// Restart the shell — refused while locked: the WlSessionLock lives in the
/// shell process, so restarting it would drop the lock and expose the
/// session.
fn restart() -> Result<()> {
    match qs_ipc(&["lock", "status"]).as_deref() {
        Some("locked") | Some("secure") => Err(VogixError::Config(
            "refusing to restart the shell while the session is locked \
             (the lock lives in the shell; restarting would unlock the screen)"
                .to_string(),
        )),
        _ => {
            let ok = Command::new("systemctl")
                .args(["--user", "restart", "vogix-desktop.service"])
                .status()
                .map(|s| s.success())
                .unwrap_or(false);
            if ok {
                Ok(())
            } else {
                Err(VogixError::Config(
                    "systemctl --user restart vogix-desktop.service failed".to_string(),
                ))
            }
        }
    }
}

fn notify(command: &NotifyCommands) -> Result<()> {
    match command {
        NotifyCommands::Dismiss { all } => {
            let verb = if *all { "dismissAll" } else { "dismiss" };
            if qs_ipc(&["notify", verb]).is_none() {
                debug!("desktop notify {verb}: no responsive shell instance");
            }
            Ok(())
        }
        NotifyCommands::Dnd { state } => dnd(state),
        NotifyCommands::History { count } => history(*count),
    }
}

fn dnd(state: &DndCommands) -> Result<()> {
    match state {
        DndCommands::On => {
            if qs_ipc(&["notify", "dndOn"]).is_none() {
                debug!("desktop notify dnd on: no responsive shell instance");
            }
        }
        DndCommands::Off => {
            if qs_ipc(&["notify", "dndOff"]).is_none() {
                debug!("desktop notify dnd off: no responsive shell instance");
            }
        }
        DndCommands::Toggle => match qs_ipc(&["notify", "dndToggle"]) {
            Some(now) => println!("dnd: {now}"),
            None => debug!("desktop notify dnd toggle: no responsive shell instance"),
        },
        DndCommands::Status => {
            // Prefer the live shell; fall back to the state file it persists,
            // so a TTY can still answer.
            let state = qs_ipc(&["notify", "dndStatus"]).unwrap_or_else(|| {
                let path = crate::config::Config::state_dir().join("desktop/dnd.json");
                match std::fs::read_to_string(path) {
                    Ok(s) if s.contains("true") => "on".to_string(),
                    _ => "off".to_string(),
                }
            });
            println!("dnd: {state}");
        }
    }
    Ok(())
}

/// Recent notifications from the shell's capped history file — read
/// directly, so it works with no shell running.
fn history(count: usize) -> Result<()> {
    let path = crate::config::Config::state_dir().join("desktop/notifications-history.json");
    let Ok(raw) = std::fs::read_to_string(&path) else {
        println!("no notification history at {}", path.display());
        return Ok(());
    };
    let entries: Vec<serde_json::Value> = serde_json::from_str(&raw).unwrap_or_default();
    for e in entries.iter().rev().take(count).rev() {
        println!(
            "{}  [{}] {}: {}",
            e["at"].as_str().unwrap_or("?"),
            e["appName"].as_str().unwrap_or("?"),
            e["summary"].as_str().unwrap_or(""),
            e["body"].as_str().unwrap_or("")
        );
    }
    Ok(())
}

fn osd(kind: &str, value: Option<u8>, muted: bool, message: Option<&str>) -> Result<()> {
    let value = value.map(|v| v.min(100) as i32).unwrap_or(-1).to_string();
    let muted = if muted { "true" } else { "false" };
    if qs_ipc(&["osd", "show", kind, &value, muted, message.unwrap_or("")]).is_none() {
        debug!("desktop osd {kind}: no responsive shell instance");
    }
    Ok(())
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

//! `vogix desktop` — desktop-shell verbs.
//!
//! The verbs are the CONTRACT between vogix and the shell: keybindings,
//! config.toml reload entries and consumer modules only ever call these. The
//! TRANSPORT behind them is an implementation detail — v1 relays to the
//! quickshell instance (`qs -c vogix ipc …`), v2 will speak the Rust
//! vogix-desktop's own socket — so swapping the shell renderer never touches
//! a caller.

use crate::cli::{
    BarCommands, DesktopCommands, DndCommands, LockCommands, NotifyCommands, PowerCommands,
    RemindCommands, SwitchCommands,
};
use crate::errors::{Result, VogixError};
use log::debug;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

pub fn handle_desktop(command: &DesktopCommands) -> Result<()> {
    match command {
        DesktopCommands::Reload => reload(),
        DesktopCommands::Check { config } => check(config.as_deref()),
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
        DesktopCommands::Background { command } => background(command),
        DesktopCommands::Osd {
            kind,
            value,
            muted,
            message,
        } => osd(kind, *value, *muted, message.as_deref()),
        DesktopCommands::Launcher { mode, query } => launcher(
            mode.as_deref().unwrap_or(""),
            query.as_deref().unwrap_or(""),
        ),
        DesktopCommands::Menu { summon } => menu(summon.as_deref().unwrap_or("")),
        DesktopCommands::Power { action } => power(action.as_ref()),
        DesktopCommands::Select { prompt } => select(prompt.as_deref().unwrap_or(""), false),
        DesktopCommands::Input { prompt } => select(prompt.as_deref().unwrap_or(""), true),
        DesktopCommands::Panel { name, close } => panel(name.as_deref(), *close),
        DesktopCommands::Nightlight { state } => switch("nightlight", state),
        DesktopCommands::StayAwake { state } => switch("stayawake", state),
        DesktopCommands::Remind { command } => remind(command),
        DesktopCommands::Gallery { close } => {
            match qs_ipc(&["gallery", if *close { "close" } else { "open" }]) {
                Some(r) => println!("{r}"),
                None => println!("no responsive shell instance"),
            }
            Ok(())
        }
    }
}

fn panel(name: Option<&str>, close: bool) -> Result<()> {
    let reply = if close {
        qs_ipc(&["panel", "close"])
    } else {
        match name {
            Some(n) => qs_ipc(&["panel", "toggle", n]),
            None => qs_ipc(&["panel", "status"]),
        }
    };
    match reply {
        Some(r) => println!("{r}"),
        None => println!("no responsive shell instance"),
    }
    Ok(())
}

fn switch(target: &str, state: &SwitchCommands) -> Result<()> {
    let verb = match state {
        SwitchCommands::On => "on",
        SwitchCommands::Off => "off",
        SwitchCommands::Toggle => "toggle",
        SwitchCommands::Status => "status",
    };
    match qs_ipc(&[target, verb]) {
        Some(r) => println!("{r}"),
        None => println!("no responsive shell instance"),
    }
    Ok(())
}

fn remind(command: &RemindCommands) -> Result<()> {
    let reply = match command {
        RemindCommands::Add { text, delay } => {
            let ms = parse_duration_ms(delay)?;
            let at = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0)
                + ms;
            qs_ipc(&["reminders", "add", text, &at.to_string()])
        }
        RemindCommands::List => qs_ipc(&["reminders", "list"]),
        RemindCommands::Clear => qs_ipc(&["reminders", "clear"]),
    };
    match reply {
        Some(r) => println!("{r}"),
        None => println!("no responsive shell instance"),
    }
    Ok(())
}

/// "10m", "1h30m", "45s", "2h" → milliseconds.
fn parse_duration_ms(spec: &str) -> Result<u64> {
    let mut total: u64 = 0;
    let mut digits = String::new();
    for c in spec.chars() {
        if c.is_ascii_digit() {
            digits.push(c);
        } else {
            let n: u64 = digits.parse().map_err(|_| {
                VogixError::Config(format!("bad duration '{spec}' (use e.g. 10m, 1h30m, 45s)"))
            })?;
            digits.clear();
            total += match c {
                's' => n * 1000,
                'm' => n * 60 * 1000,
                'h' => n * 60 * 60 * 1000,
                'd' => n * 24 * 60 * 60 * 1000,
                _ => {
                    return Err(VogixError::Config(format!(
                        "bad duration unit '{c}' in '{spec}' (s, m, h, d)"
                    )));
                }
            };
        }
    }
    if !digits.is_empty() {
        // A bare number means minutes.
        total += digits
            .parse::<u64>()
            .map_err(|_| VogixError::Config(format!("bad duration '{spec}'")))?
            * 60
            * 1000;
    }
    if total == 0 {
        return Err(VogixError::Config(format!(
            "duration '{spec}' is zero (use e.g. 10m, 1h30m, 45s)"
        )));
    }
    Ok(total)
}

/// Validate desktop.json's surface tokens against the praxis ontology:
/// every `{ slot, alpha }` names one of the 16 semantic keys
/// (Vogix16Semantic — praxis is the authority, nothing re-encoded here),
/// alpha stays in [0,1], and when the current theme's contract file is
/// readable, every referenced slot actually resolves in its semantic map
/// (the SurfaceSlotsResolvable obligation, checked against the LIVE
/// palette). Hard failure on any violation — the file is Nix-generated,
/// so an error here is a generator bug, not user error.
fn check(config: Option<&str>) -> Result<()> {
    use pr4xis::category::FinitelyGenerated;
    use pr4xis_domains::applied::hmi::theming::schemes::Vogix16Semantic;

    let path = config
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| crate::config::Config::state_dir().join("desktop.json"));
    let raw = std::fs::read_to_string(&path)
        .map_err(|e| VogixError::Config(format!("cannot read {}: {e}", path.display())))?;
    let doc: serde_json::Value = serde_json::from_str(&raw)
        .map_err(|e| VogixError::Config(format!("{} is not valid JSON: {e}", path.display())))?;

    let keys: Vec<String> = Vogix16Semantic::variants()
        .iter()
        .map(|s| s.key().to_string())
        .collect();

    // The live palette, when a theme with the desktop contract is active.
    let semantic: Option<serde_json::Value> = std::fs::read_to_string(
        crate::config::Config::state_dir().join("current-theme/vogix-desktop/theme.json"),
    )
    .ok()
    .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
    .and_then(|t| t.get("semantic").cloned());

    let mut errors: Vec<String> = Vec::new();
    let mut tokens = 0usize;
    let empty = serde_json::Map::new();
    let surfaces = doc
        .get("surfaces")
        .and_then(|s| s.as_object())
        .unwrap_or(&empty);
    for (surface, table) in surfaces {
        let Some(table) = table.as_object() else {
            errors.push(format!("surface '{surface}' is not an object"));
            continue;
        };
        for (token, value) in table {
            tokens += 1;
            let slot = value.get("slot").and_then(|s| s.as_str()).unwrap_or("");
            if !keys.iter().any(|k| k == slot) {
                errors.push(format!(
                    "{surface}.{token}: slot '{slot}' is not one of the 16 semantic keys"
                ));
                continue;
            }
            if let Some(alpha) = value.get("alpha").and_then(|a| a.as_f64())
                && !(0.0..=1.0).contains(&alpha)
            {
                errors.push(format!("{surface}.{token}: alpha {alpha} outside [0,1]"));
            }
            if let Some(sem) = &semantic
                && sem.get(slot).and_then(|v| v.as_str()).is_none()
            {
                errors.push(format!(
                    "{surface}.{token}: slot '{slot}' does not resolve in the current theme"
                ));
            }
        }
    }

    if errors.is_empty() {
        println!(
            "desktop.json OK — {} surfaces, {tokens} tokens, every slot {}",
            surfaces.len(),
            if semantic.is_some() {
                "resolves in the current theme"
            } else {
                "names a valid semantic key (no active theme contract to resolve against)"
            }
        );
        Ok(())
    } else {
        for e in &errors {
            eprintln!("✗ {e}");
        }
        Err(VogixError::Config(format!(
            "desktop.json failed validation with {} error(s)",
            errors.len()
        )))
    }
}

fn launcher(mode: &str, query: &str) -> Result<()> {
    match qs_ipc(&["launcher", "open", mode, query]) {
        Some(r) => println!("{r}"),
        None => println!("no responsive shell instance"),
    }
    Ok(())
}

fn menu(summon: &str) -> Result<()> {
    match qs_ipc(&["launcher", "menu", summon]) {
        Some(r) => println!("{r}"),
        None => println!("no responsive shell instance"),
    }
    Ok(())
}

/// With no action: toggle the shell's power menu. With one: run it directly —
/// systemd verbs work without a shell, `lock` keeps its loud-failure path.
fn power(action: Option<&PowerCommands>) -> Result<()> {
    let Some(action) = action else {
        match qs_ipc(&["power", "toggle"]) {
            Some(r) => println!("{r}"),
            None => println!("no responsive shell instance"),
        }
        return Ok(());
    };
    match action {
        PowerCommands::Lock => lock(None),
        PowerCommands::Logout => {
            crate::commands::hypr::handle_hypr(&crate::cli::HyprCommands::Dispatch {
                action: "exit".to_string(),
            })
        }
        PowerCommands::Suspend | PowerCommands::Reboot | PowerCommands::Poweroff => {
            let verb = match action {
                PowerCommands::Suspend => "suspend",
                PowerCommands::Reboot => "reboot",
                _ => "poweroff",
            };
            let ok = Command::new("systemctl")
                .arg(verb)
                .status()
                .map(|s| s.success())
                .unwrap_or(false);
            if ok {
                Ok(())
            } else {
                Err(VogixError::Config(format!("systemctl {verb} failed")))
            }
        }
    }
}

/// dmenu mode. Items come in on stdin (one per line; `input` takes none),
/// go to the shell through a session file, and the choice comes back through
/// a result file the shell writes on every exit path — pick, cancel, close.
/// dmenu convention on cancel: exit 1, nothing on stdout.
fn select(prompt: &str, text_only: bool) -> Result<()> {
    use std::io::Read;

    let items: Vec<String> = if text_only {
        Vec::new()
    } else {
        let mut raw = String::new();
        std::io::stdin()
            .read_to_string(&mut raw)
            .map_err(|e| VogixError::Config(format!("cannot read stdin: {e}")))?;
        raw.lines()
            .filter(|l| !l.is_empty())
            .map(str::to_string)
            .collect()
    };

    let dir = crate::config::Config::state_dir().join("desktop");
    std::fs::create_dir_all(&dir)
        .map_err(|e| VogixError::Config(format!("cannot create {}: {e}", dir.display())))?;
    let id = format!(
        "{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0)
    );
    let items_path = dir.join(format!("select-{id}.json"));
    let result_path = dir.join(format!("select-{id}.result"));
    std::fs::write(
        &items_path,
        serde_json::json!({ "items": items }).to_string(),
    )
    .map_err(|e| VogixError::Config(format!("cannot write {}: {e}", items_path.display())))?;

    let verb = if text_only { "inputText" } else { "select" };
    let opened = qs_ipc(&["launcher", verb, &id, prompt]);
    if opened.is_none() {
        let _ = std::fs::remove_file(&items_path);
        return Err(VogixError::Config(
            "cannot open the picker: no responsive vogix shell instance".to_string(),
        ));
    }

    // Poll for the result; the human decides the pace, so the bound is only
    // there to reap a session whose shell died mid-pick.
    let deadline = Instant::now() + Duration::from_secs(600);
    let choice = loop {
        if let Ok(raw) = std::fs::read_to_string(&result_path) {
            break serde_json::from_str::<serde_json::Value>(&raw).ok();
        }
        if Instant::now() >= deadline {
            let _ = std::fs::remove_file(&items_path);
            return Err(VogixError::Config(
                "picker session timed out with no result".to_string(),
            ));
        }
        std::thread::sleep(Duration::from_millis(50));
    };
    let _ = std::fs::remove_file(&items_path);
    let _ = std::fs::remove_file(&result_path);

    match choice
        .as_ref()
        .and_then(|v| v.get("choice"))
        .and_then(|c| c.as_str())
    {
        Some(picked) => {
            println!("{picked}");
            Ok(())
        }
        None => std::process::exit(1),
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

fn background(command: &crate::cli::BackgroundCommands) -> Result<()> {
    use crate::cli::BackgroundCommands as B;
    let reply = match command {
        B::Set { path } => {
            // The shell renders whatever it is handed; catch a missing file
            // here, where the caller can read the error.
            let p = std::path::Path::new(path);
            if !p.is_file() {
                return Err(VogixError::Config(format!(
                    "background image not found: {path}"
                )));
            }
            let canonical = p
                .canonicalize()
                .map_err(|e| VogixError::Config(format!("cannot resolve {path}: {e}")))?;
            qs_ipc(&["background", "set", &canonical.to_string_lossy()])
        }
        B::Next => qs_ipc(&["background", "next"]),
        B::Clear => qs_ipc(&["background", "clear"]),
        B::Status => qs_ipc(&["background", "status"]),
    };
    match reply {
        Some(r) => println!("{r}"),
        None => println!("no responsive shell instance"),
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

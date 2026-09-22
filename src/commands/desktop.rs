//! `vogix desktop` — desktop-shell verbs.
//!
//! The verbs are the CONTRACT between vogix and the shell: keybindings,
//! config.toml reload entries and consumer modules only ever call these. The
//! TRANSPORT behind them is an implementation detail — v1 relays to the
//! quickshell instance (`qs -c vogix ipc …`), v2 will speak the Rust
//! vogix-desktop's own socket — so swapping the shell renderer never touches
//! a caller.

use crate::cli::{
    BarCommands, CustomCommands, DesktopCommands, DndCommands, LockCommands, NotifyCommands,
    PowerCommands, RemindCommands, SwitchCommands,
};
use crate::errors::{Result, VogixError};
use log::debug;
use serde_json::Value;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

pub fn handle_desktop(command: &DesktopCommands) -> Result<()> {
    match command {
        DesktopCommands::Reload => reload(),
        DesktopCommands::Check { config } => check(config.as_deref()),
        DesktopCommands::Status => status(),
        DesktopCommands::Meters => meters(),
        DesktopCommands::Stats => relay_status("stats"),
        DesktopCommands::Privacy => relay_status("privacy"),
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
        DesktopCommands::Custom { command } => custom(command),
        DesktopCommands::Keyboard => {
            match qs_ipc(&["keyboard", "status"]) {
                Some(r) => println!("{r}"),
                None => println!("no responsive shell instance"),
            }
            Ok(())
        }
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
        RemindCommands::Add { text, delay_ms } => {
            let at = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0)
                .saturating_add(*delay_ms);
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

fn custom(command: &CustomCommands) -> Result<()> {
    let reply = match command {
        CustomCommands::Refresh { name } => qs_ipc(&["custom", "refresh", name]),
        CustomCommands::Status { name } => qs_ipc(&["custom", "status", name]),
    };
    match reply {
        // A name desktop.json does not define is the caller's error.
        Some(r) if r.starts_with("unknown custom cell") => Err(VogixError::Config(r)),
        Some(r) => {
            println!("{r}");
            Ok(())
        }
        None => {
            println!("no responsive shell instance");
            Ok(())
        }
    }
}

/// Validate desktop.json; see [`validate`] for what is checked. Hard
/// failure on any violation — the file is Nix-generated, so an error here
/// is a generator bug, not user error.
fn check(config: Option<&str>) -> Result<()> {
    let path = config
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| crate::config::Config::state_dir().join("desktop.json"));
    let raw = std::fs::read_to_string(&path)
        .map_err(|e| VogixError::Config(format!("cannot read {}: {e}", path.display())))?;
    let doc: Value = serde_json::from_str(&raw)
        .map_err(|e| VogixError::Config(format!("{} is not valid JSON: {e}", path.display())))?;

    // The live palette, when a theme with the desktop contract is active.
    let semantic: Option<Value> = std::fs::read_to_string(
        crate::config::Config::state_dir().join("current-theme/vogix-desktop/theme.json"),
    )
    .ok()
    .and_then(|s| serde_json::from_str::<Value>(&s).ok())
    .and_then(|t| t.get("semantic").cloned());

    let report = validate(&doc, semantic.as_ref());
    if report.errors.is_empty() {
        println!(
            "desktop.json OK — {} surfaces, {} tokens, {} bar widgets, {} custom cells, {} menu commands, every slot {}",
            report.surfaces,
            report.tokens,
            report.widgets,
            report.custom_cells,
            report.menu_commands,
            if semantic.is_some() {
                "resolves in the current theme"
            } else {
                "names a valid semantic key (no active theme contract to resolve against)"
            }
        );
        Ok(())
    } else {
        for e in &report.errors {
            eprintln!("✗ {e}");
        }
        Err(VogixError::Config(format!(
            "desktop.json failed validation with {} error(s)",
            report.errors.len()
        )))
    }
}

/// What one validation pass over a desktop.json found.
#[derive(Debug, Default)]
struct CheckReport {
    errors: Vec<String>,
    surfaces: usize,
    tokens: usize,
    widgets: usize,
    custom_cells: usize,
    menu_commands: usize,
}

/// The desktop.json schema the shell reads (the four-edge `bars` table).
const DESKTOP_SCHEMA: u64 = 2;

/// The whole desktop.json contract, checked without touching the
/// filesystem: the schema, surface tokens, the four-bar widget table, the
/// custom cells, and the launcher menu's commands. `semantic` is the live
/// theme's semantic map, when one is active.
fn validate(doc: &Value, semantic: Option<&Value>) -> CheckReport {
    let mut report = CheckReport::default();
    match doc.get("schema") {
        Some(v) if v.as_u64() == Some(DESKTOP_SCHEMA) => {}
        found => report.errors.push(format!(
            "schema {} is not supported (the shell reads schema {DESKTOP_SCHEMA}; rebuild to regenerate desktop.json)",
            found.map_or_else(|| "(missing)".to_string(), Value::to_string)
        )),
    }
    check_surfaces(doc, semantic, &mut report);
    check_bar_widgets(doc, &mut report);
    check_custom_cells(doc, &mut report);
    check_menu_commands(doc, &mut report);
    report
}

/// Every `{ slot, alpha }` names one of the 16 semantic keys
/// (Vogix16Semantic — praxis is the authority, nothing re-encoded here),
/// alpha stays in [0,1], and when the current theme's contract file is
/// readable, every referenced slot actually resolves in its semantic map
/// (the SurfaceSlotsResolvable obligation, checked against the LIVE
/// palette).
fn check_surfaces(doc: &Value, semantic: Option<&Value>, report: &mut CheckReport) {
    use pr4xis::category::FinitelyGenerated;
    use pr4xis_domains::applied::hmi::theming::schemes::Vogix16Semantic;

    let keys: Vec<String> = Vogix16Semantic::variants()
        .iter()
        .map(|s| s.key().to_string())
        .collect();

    let Some(surfaces) = doc.get("surfaces").and_then(|s| s.as_object()) else {
        return;
    };
    report.surfaces = surfaces.len();
    for (surface, table) in surfaces {
        let Some(table) = table.as_object() else {
            report
                .errors
                .push(format!("surface '{surface}' is not an object"));
            continue;
        };
        for (token, value) in table {
            report.tokens += 1;
            let slot = value.get("slot").and_then(|s| s.as_str()).unwrap_or("");
            if !keys.iter().any(|k| k == slot) {
                report.errors.push(format!(
                    "{surface}.{token}: slot '{slot}' is not one of the 16 semantic keys"
                ));
                continue;
            }
            if let Some(alpha) = value.get("alpha").and_then(|a| a.as_f64())
                && !(0.0..=1.0).contains(&alpha)
            {
                report
                    .errors
                    .push(format!("{surface}.{token}: alpha {alpha} outside [0,1]"));
            }
            if let Some(sem) = semantic
                && sem.get(slot).and_then(|v| v.as_str()).is_none()
            {
                report.errors.push(format!(
                    "{surface}.{token}: slot '{slot}' does not resolve in the current theme"
                ));
            }
        }
    }
}

/// The shell's bar widget registry (desktop/Bar/widgets/registry.json), the
/// same file Section.qml resolves names through and the Nix layout options
/// take their name type from: compiled in, so this check always agrees
/// with the shell it ships beside.
const WIDGET_REGISTRY_JSON: &str = include_str!("../../desktop/Bar/widgets/registry.json");

#[derive(Debug, serde::Deserialize)]
struct WidgetRegistry {
    widgets: std::collections::BTreeMap<String, RegisteredWidget>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RegisteredWidget {
    /// The component in desktop/Bar/widgets that renders the name (the
    /// registry tests hold it to an existing file; the check itself needs
    /// only the name and the flag).
    #[cfg_attr(not(test), expect(dead_code, reason = "read by the registry tests"))]
    component: String,
    /// Reads only horizontally, so never renders on a vertical bar.
    #[serde(default)]
    horizontal_only: bool,
}

fn widget_registry() -> &'static WidgetRegistry {
    static REGISTRY: std::sync::LazyLock<WidgetRegistry> = std::sync::LazyLock::new(|| {
        serde_json::from_str(WIDGET_REGISTRY_JSON)
            .expect("desktop/Bar/widgets/registry.json is compiled in and pinned by a test")
    });
    &REGISTRY
}

/// A bar places the custom cell `<name>` as `custom/<name>`.
const CUSTOM_PREFIX: &str = "custom/";

/// The custom cells (`custom.<name>`): a name one path segment of letters,
/// digits, `-` and `_`; a command the shell can run; and every field in
/// the shape the shell reads.
fn check_custom_cells(doc: &Value, report: &mut CheckReport) {
    let Some(cells) = doc.get("custom") else {
        return;
    };
    let Some(cells) = cells.as_object() else {
        report.errors.push("custom: not an object".to_string());
        return;
    };
    for (name, cell) in cells {
        let at = format!("custom.{name}");
        if name.is_empty()
            || !name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        {
            report.errors.push(format!(
                "{at}: a cell name may hold only letters, digits, '-' and '_'"
            ));
        }
        let Some(cell) = cell.as_object() else {
            report.errors.push(format!("{at}: not an object"));
            continue;
        };
        report.custom_cells += 1;
        let mut problem = |field: &str, why: String| {
            report.errors.push(format!("{at}.{field}: {why}"));
        };
        match cell.get("command") {
            Some(Value::String(command)) => {
                if let Some(why) = shell_command_problem(command) {
                    problem("command", why);
                }
            }
            _ => problem("command", "a command string is required".to_string()),
        }
        match cell.get("onClick") {
            None | Some(Value::Null) => {}
            Some(Value::String(command)) => {
                if let Some(why) = shell_command_problem(command) {
                    problem("onClick", why);
                }
            }
            Some(_) => problem("onClick", "not a string".to_string()),
        }
        match cell.get("output") {
            None => {}
            Some(Value::String(o)) if o == "text" || o == "json" => {}
            Some(other) => problem("output", format!("{other} is not \"text\" or \"json\"")),
        }
        match cell.get("interval") {
            None | Some(Value::Null) => {}
            Some(v) if v.as_u64().is_some_and(|s| s > 0) => {}
            Some(other) => problem(
                "interval",
                format!("{other} is not a positive number of seconds"),
            ),
        }
        match cell.get("watch") {
            None => {}
            Some(Value::Array(paths)) => {
                for p in paths {
                    if !p.as_str().is_some_and(|p| p.starts_with('/')) {
                        problem("watch", format!("{p} is not an absolute path"));
                    }
                }
            }
            Some(_) => problem("watch", "not a list of paths".to_string()),
        }
        if cell.get("stream").is_some_and(|v| !v.is_boolean()) {
            problem("stream", "not a boolean".to_string());
        }
        for field in ["title", "widest"] {
            if cell
                .get(field)
                .is_some_and(|v| !v.is_string() && !v.is_null())
            {
                problem(field, "not a string".to_string());
            }
        }
    }
}

/// The four-bar table: every named widget must be one the shell's
/// registry knows or a `custom/<name>` cell desktop.json defines, and
/// horizontal-only widgets may not appear on a vertical bar.
fn check_bar_widgets(doc: &Value, report: &mut CheckReport) {
    let Some(bars) = doc.get("bars").and_then(|b| b.as_object()) else {
        return;
    };
    let custom = doc.get("custom").and_then(|c| c.as_object());
    for (edge, bar) in bars {
        let vertical = edge == "left" || edge == "right";
        let layout = bar.get("layout").and_then(|l| l.as_object());
        for section in ["start", "center", "end"] {
            let names = layout
                .and_then(|l| l.get(section))
                .and_then(|w| w.as_array());
            for w in names.into_iter().flatten() {
                report.widgets += 1;
                let name = w.as_str().unwrap_or("");
                if let Some(cell) = name.strip_prefix(CUSTOM_PREFIX) {
                    if !custom.is_some_and(|c| c.contains_key(cell)) {
                        report.errors.push(format!(
                            "bars.{edge}.layout.{section}: '{name}' names no cell under `custom`"
                        ));
                    }
                } else {
                    match widget_registry().widgets.get(name) {
                        None => report.errors.push(format!(
                            "bars.{edge}.layout.{section}: unknown widget '{name}'"
                        )),
                        Some(w) if vertical && w.horizontal_only => report.errors.push(format!(
                            "bars.{edge}.layout.{section}: '{name}' is horizontal-only and cannot render on a vertical bar"
                        )),
                        Some(_) => {}
                    }
                }
            }
        }
    }
}

/// The launcher menu's commands. The shell runs every `action` and `when`
/// through `sh -c`, detached, so a malformed one fails invisibly at click
/// time; here it fails the check instead. Entries nest one level
/// (`submenu`).
fn check_menu_commands(doc: &Value, report: &mut CheckReport) {
    let Some(menu) = doc.pointer("/launcher/menu").and_then(|m| m.as_array()) else {
        return;
    };
    for (i, entry) in menu.iter().enumerate() {
        let at = format!("launcher.menu.{}", menu_entry_id(entry, i));
        check_menu_entry(&at, entry, report);
        let submenu = entry.get("submenu").and_then(|s| s.as_array());
        for (j, sub) in submenu.into_iter().flatten().enumerate() {
            let sub_at = format!("{at}.submenu.{}", menu_entry_id(sub, j));
            check_menu_entry(&sub_at, sub, report);
        }
    }
}

fn menu_entry_id(entry: &Value, index: usize) -> String {
    entry
        .get("id")
        .and_then(|v| v.as_str())
        .map(str::to_string)
        .unwrap_or_else(|| format!("#{index}"))
}

fn check_menu_entry(at: &str, entry: &Value, report: &mut CheckReport) {
    for field in ["action", "when"] {
        match entry.get(field) {
            None | Some(Value::Null) => {}
            Some(Value::String(command)) => {
                report.menu_commands += 1;
                if let Some(problem) = shell_command_problem(command) {
                    report.errors.push(format!("{at}.{field}: {problem}"));
                }
            }
            Some(_) => report.errors.push(format!("{at}.{field}: not a string")),
        }
    }
}

/// Why `sh -c <command>` would certainly fail, if it would. Every command
/// must be valid shell quoting; one whose program is `vogix` must also
/// parse with this binary's own CLI. A command that uses shell syntax
/// beyond quoting (operators, redirections, expansions) is checked for
/// quoting only: its words are not a single argv.
fn shell_command_problem(command: &str) -> Option<String> {
    use clap::Parser;

    let Some(words) = shlex::split(command) else {
        return Some(format!("`{command}` has unbalanced shell quoting"));
    };
    let Some(program) = words.first() else {
        return Some("empty command".to_string());
    };
    let runs_vogix = program == "vogix" || program.ends_with("/vogix");
    let beyond_quoting = command.contains([';', '&', '|', '<', '>', '$', '`', '(', ')', '\n']);
    if !runs_vogix || beyond_quoting {
        return None;
    }
    match crate::cli::Cli::try_parse_from(&words) {
        Ok(_) => None,
        Err(e)
            if matches!(
                e.kind(),
                clap::error::ErrorKind::DisplayHelp | clap::error::ErrorKind::DisplayVersion
            ) =>
        {
            None
        }
        Err(e) => {
            let rendered = e.to_string();
            let reason = rendered
                .lines()
                .next()
                .unwrap_or_default()
                .trim_start_matches("error: ");
            Some(format!(
                "`{command}` is not a valid vogix command: {reason}"
            ))
        }
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
    // Transport function is `flash`: "show" is quickshell's own `ipc call
    // <target> show` introspection verb and never reaches a handler.
    if qs_ipc(&["osd", "flash", kind, &value, muted, message.unwrap_or("")]).is_none() {
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

/// What the HUD samples right now, as the shell reports it.
fn meters() -> Result<()> {
    match qs_ipc(&["meters", "status"]) {
        Some(state) => println!("{state}"),
        None => println!("no responsive shell instance"),
    }
    Ok(())
}

/// A read-only status verb: print what the shell's `<target> status` says.
fn relay_status(target: &str) -> Result<()> {
    match qs_ipc(&[target, "status"]) {
        Some(state) => println!("{state}"),
        None => println!("no responsive shell instance"),
    }
    Ok(())
}

fn bar(command: &BarCommands) -> Result<()> {
    // Transport name is `unhide`: "show" is quickshell's own `ipc call
    // <target> show` introspection verb and never reaches a handler.
    let (verb, edge) = match command {
        BarCommands::Show { edge } => ("unhide", Some(edge.as_str())),
        BarCommands::Hide { edge } => ("hide", Some(edge.as_str())),
        BarCommands::Toggle { edge } => ("toggle", Some(edge.as_str())),
        BarCommands::Status => ("status", None),
    };
    let args: Vec<&str> = match edge {
        Some(e) => vec!["bar", verb, e],
        None => vec!["bar", verb],
    };
    match qs_ipc(&args) {
        Some(out) => {
            if !out.is_empty() {
                println!("{out}");
            }
        }
        None => debug!("desktop bar {verb}: no responsive shell instance"),
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

#[cfg(test)]
mod tests {
    use super::*;

    /// The default desktop.json as the home-manager module renders it; the
    /// `desktop-options` flake check keeps this pin equal to the render.
    const DEFAULT_DESKTOP_JSON: &str =
        include_str!("../../nix/modules/desktop/desktop-json.pin.json");

    fn default_doc() -> Value {
        serde_json::from_str(DEFAULT_DESKTOP_JSON).expect("the pin is valid JSON")
    }

    fn menu_doc(action: &str) -> Value {
        serde_json::json!({
            "launcher": { "menu": [ { "id": "probe", "action": action, "when": null, "submenu": [] } ] }
        })
    }

    #[test]
    fn the_default_desktop_json_validates() {
        let report = validate(&default_doc(), None);
        assert!(report.errors.is_empty(), "{:#?}", report.errors);
        assert!(report.widgets > 0 && report.tokens > 0);
    }

    fn custom_doc(cell: Value, placed: &str) -> Value {
        serde_json::json!({
            "schema": 2,
            "bars": { "top": { "enable": true, "size": 32,
                "layout": { "start": [placed], "center": [], "end": [] } } },
            "custom": { "probe": cell }
        })
    }

    #[test]
    fn a_defined_custom_cell_validates_on_any_bar() {
        let cell = serde_json::json!({
            "title": "UPD", "command": "checkupdates | wc -l", "output": "text",
            "interval": 3600, "watch": ["/run/user/1000/state"], "stream": false,
            "onClick": "vogix desktop custom refresh probe", "widest": "999"
        });
        let report = validate(&custom_doc(cell.clone(), "custom/probe"), None);
        assert!(report.errors.is_empty(), "{:#?}", report.errors);
        assert_eq!(report.custom_cells, 1);

        let mut doc = custom_doc(cell, "custom/probe");
        doc["bars"]["left"] = doc["bars"]["top"].clone();
        assert!(validate(&doc, None).errors.is_empty());
    }

    #[test]
    fn a_custom_placement_must_name_a_defined_cell() {
        let cell = serde_json::json!({ "command": "date" });
        let report = validate(&custom_doc(cell, "custom/nope"), None);
        assert_eq!(report.errors.len(), 1, "{:#?}", report.errors);
        assert!(report.errors[0].contains("'custom/nope' names no cell"));
    }

    #[test]
    fn malformed_custom_cells_fail_the_check() {
        for (cell, field) in [
            (serde_json::json!({}), "command"),
            (serde_json::json!({ "command": "echo 'open" }), "command"),
            (
                serde_json::json!({ "command": "vogix desktop custom poke x" }),
                "command",
            ),
            (
                serde_json::json!({ "command": "date", "output": "yaml" }),
                "output",
            ),
            (
                serde_json::json!({ "command": "date", "interval": 0 }),
                "interval",
            ),
            (
                serde_json::json!({ "command": "date", "watch": ["relative/file"] }),
                "watch",
            ),
            (
                serde_json::json!({ "command": "date", "stream": "yes" }),
                "stream",
            ),
            (
                serde_json::json!({ "command": "date", "onClick": 7 }),
                "onClick",
            ),
            (
                serde_json::json!({ "command": "date", "widest": 3 }),
                "widest",
            ),
        ] {
            let report = validate(&custom_doc(cell.clone(), "custom/probe"), None);
            assert_eq!(report.errors.len(), 1, "{cell}: {:#?}", report.errors);
            assert!(
                report.errors[0].starts_with(&format!("custom.probe.{field}:")),
                "{cell}: {:#?}",
                report.errors
            );
        }

        let doc = serde_json::json!({ "schema": 2, "custom": { "a/b": { "command": "date" } } });
        let report = validate(&doc, None);
        assert_eq!(report.errors.len(), 1, "{:#?}", report.errors);
        assert!(report.errors[0].starts_with("custom.a/b:"));
    }

    const SECTION_QML: &str = include_str!("../../desktop/Bar/Section.qml");

    /// The shell's Section routes the same prefix this check accepts to the
    /// custom cell.
    #[test]
    fn the_section_places_custom_cells() {
        assert!(SECTION_QML.contains(&format!(".startsWith(\"{CUSTOM_PREFIX}\")")));
        assert!(SECTION_QML.contains(&format!(".slice({})", CUSTOM_PREFIX.len())));
        assert!(SECTION_QML.contains("widgets/CustomCell.qml"));
    }

    /// Every registry entry names a component that exists, and the flag is
    /// spelled the way the registry's readers look it up (an unknown field
    /// fails the parse, so a misspelt flag cannot read as false).
    #[test]
    fn the_widget_registry_names_existing_components() {
        let registry = widget_registry();
        assert!(!registry.widgets.is_empty());
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("desktop/Bar/widgets");
        for (name, w) in &registry.widgets {
            let file = dir.join(format!("{}.qml", w.component));
            assert!(file.is_file(), "{name}: {} does not exist", file.display());
        }
        assert!(registry.widgets["window"].horizontal_only);
        assert!(!registry.widgets["clock"].horizontal_only);
    }

    /// Section.qml resolves every name through the registry rather than a
    /// list of its own, so the shell cannot drift from this check.
    #[test]
    fn the_section_resolves_names_through_the_registry() {
        const REGISTRY_QML: &str = include_str!("../../desktop/Services/WidgetRegistry.qml");
        assert!(REGISTRY_QML.contains("Qt.resolvedUrl(\"../Bar/widgets/registry.json\")"));
        assert!(SECTION_QML.contains("WidgetRegistry.widgets[name]"));
        assert!(SECTION_QML.contains("entry.horizontalOnly"));
        assert!(
            !SECTION_QML.contains("case \""),
            "Section.qml carries its own widget names again"
        );
    }

    #[test]
    fn unknown_and_misplaced_widgets_fail_the_check() {
        let doc = |edge: &str, name: &str| {
            serde_json::json!({
                "schema": 2,
                "bars": { edge: { "enable": true, "size": 32,
                    "layout": { "start": [name], "center": [], "end": [] } } }
            })
        };
        assert!(validate(&doc("top", "window"), None).errors.is_empty());
        assert!(validate(&doc("left", "clock"), None).errors.is_empty());
        let unknown = validate(&doc("top", "clokc"), None).errors;
        assert_eq!(unknown, ["bars.top.layout.start: unknown widget 'clokc'"]);
        let misplaced = validate(&doc("right", "window"), None).errors;
        assert_eq!(misplaced.len(), 1, "{misplaced:#?}");
        assert!(misplaced[0].contains("'window' is horizontal-only"));
    }

    #[test]
    fn only_schema_2_is_accepted() {
        for doc in [
            serde_json::json!({ "schema": 1, "bar": { "enable": true } }),
            serde_json::json!({ "schema": "2" }),
            serde_json::json!({}),
        ] {
            let report = validate(&doc, None);
            assert_eq!(report.errors.len(), 1, "{doc}: {:#?}", report.errors);
            assert!(report.errors[0].starts_with("schema "));
        }
        assert!(
            validate(&serde_json::json!({ "schema": 2 }), None)
                .errors
                .is_empty()
        );
    }

    #[test]
    fn every_default_menu_command_parses_with_the_cli() {
        let doc = default_doc();
        let mut report = CheckReport::default();
        check_menu_commands(&doc, &mut report);
        assert!(report.errors.is_empty(), "{:#?}", report.errors);
        // Every root entry carries an action, so each reached the parser:
        // a pin that lost its menu cannot pass by having nothing to check.
        let entries = doc["launcher"]["menu"].as_array().map_or(0, Vec::len);
        assert!(entries > 0);
        assert!(report.menu_commands >= entries);
    }

    #[test]
    fn a_positional_remind_delay_fails_the_check() {
        let mut report = CheckReport::default();
        check_menu_commands(
            &menu_doc("vogix desktop remind add 'Reminder' 10m"),
            &mut report,
        );
        assert_eq!(report.errors.len(), 1, "{:#?}", report.errors);
        assert!(report.errors[0].starts_with("launcher.menu.probe.action:"));

        let mut report = CheckReport::default();
        check_menu_commands(
            &menu_doc("vogix desktop remind add 'Reminder' --in 10m"),
            &mut report,
        );
        assert!(report.errors.is_empty(), "{:#?}", report.errors);
    }

    #[test]
    fn menu_commands_are_checked_in_submenus_and_guards() {
        let doc = serde_json::json!({
            "launcher": { "menu": [ {
                "id": "outer",
                "action": null,
                "when": "vogix desktop no-such-verb",
                "submenu": [ { "id": "inner", "action": "vogix theme set --bogus", "when": null } ]
            } ] }
        });
        let mut report = CheckReport::default();
        check_menu_commands(&doc, &mut report);
        assert_eq!(report.menu_commands, 2);
        assert_eq!(report.errors.len(), 2, "{:#?}", report.errors);
        assert!(report.errors[0].starts_with("launcher.menu.outer.when:"));
        assert!(report.errors[1].starts_with("launcher.menu.outer.submenu.inner.action:"));
    }

    #[test]
    fn broken_quoting_and_empty_commands_fail_whatever_they_run() {
        for bad in ["notify-send 'unterminated", "   "] {
            let mut report = CheckReport::default();
            check_menu_commands(&menu_doc(bad), &mut report);
            assert_eq!(report.errors.len(), 1, "{bad:?}: {:#?}", report.errors);
        }
    }

    #[test]
    fn foreign_and_compound_commands_are_not_argv_parsed() {
        for ok in [
            "notify-send hello --whatever",
            "vogix desktop bogus && notify-send done",
            "vogix desktop remind add \"$(date)\" --in 5m",
        ] {
            let mut report = CheckReport::default();
            check_menu_commands(&menu_doc(ok), &mut report);
            assert!(report.errors.is_empty(), "{ok:?}: {:#?}", report.errors);
        }
    }
}

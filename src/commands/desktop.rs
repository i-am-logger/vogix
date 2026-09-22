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

/// One call on the shell's IPC surface: `qs -c vogix ipc call TARGET
/// FUNCTION ARGS…`. Every verb reaches the shell through these, so what a
/// verb asks of the shell is plain data.
#[derive(Debug, Clone, PartialEq, Eq)]
struct IpcCall {
    target: &'static str,
    function: &'static str,
    args: Vec<String>,
}

impl IpcCall {
    fn new(target: &'static str, function: &'static str) -> Self {
        Self {
            target,
            function,
            args: Vec::new(),
        }
    }

    fn arg(mut self, value: impl Into<String>) -> Self {
        self.args.push(value.into());
        self
    }
}

/// The shell instance the verbs talk to.
trait Shell {
    /// The instance's trimmed reply to `call`, or None when no responsive
    /// instance answered.
    fn call(&mut self, call: &IpcCall) -> Option<String>;
}

/// The running quickshell instance of the `vogix` config, over `qs ipc`.
struct Quickshell;

impl Shell for Quickshell {
    fn call(&mut self, call: &IpcCall) -> Option<String> {
        qs_ipc(call)
    }
}

pub fn handle_desktop(command: &DesktopCommands) -> Result<()> {
    run(command, &mut Quickshell)
}

fn run(command: &DesktopCommands, shell: &mut dyn Shell) -> Result<()> {
    match command {
        DesktopCommands::Reload => reload(shell),
        DesktopCommands::Check { config } => check(config.as_deref()),
        DesktopCommands::Status => status(shell),
        DesktopCommands::Meters => print_reply(shell, &IpcCall::new("meters", "status")),
        DesktopCommands::Stats => print_reply(shell, &IpcCall::new("stats", "status")),
        DesktopCommands::Privacy => print_reply(shell, &IpcCall::new("privacy", "status")),
        DesktopCommands::Bar { command } => bar(shell, command),
        DesktopCommands::Notify { command } => notify(shell, command),
        DesktopCommands::Lock {
            wait_secure,
            command,
        } => match command {
            Some(LockCommands::Status) => lock_status(shell),
            None => lock(shell, *wait_secure),
        },
        DesktopCommands::Restart => restart(shell),
        DesktopCommands::Background { command } => background(shell, command),
        DesktopCommands::Osd {
            kind,
            value,
            muted,
            message,
        } => osd(shell, kind, *value, *muted, message.as_deref()),
        DesktopCommands::Launcher { mode, query } => print_reply(
            shell,
            &IpcCall::new("launcher", "open")
                .arg(mode.as_deref().unwrap_or(""))
                .arg(query.as_deref().unwrap_or("")),
        ),
        DesktopCommands::Menu { summon } => print_reply(
            shell,
            &IpcCall::new("launcher", "menu").arg(summon.as_deref().unwrap_or("")),
        ),
        DesktopCommands::Power { action } => power(shell, action.as_ref()),
        DesktopCommands::Select { prompt } => select(shell, prompt.as_deref().unwrap_or(""), false),
        DesktopCommands::Input { prompt } => select(shell, prompt.as_deref().unwrap_or(""), true),
        DesktopCommands::Panel { name, close } => {
            print_reply(shell, &panel_call(name.as_deref(), *close))
        }
        DesktopCommands::Nightlight { state } => {
            print_reply(shell, &switch_call("nightlight", state))
        }
        DesktopCommands::StayAwake { state } => {
            print_reply(shell, &switch_call("stayawake", state))
        }
        DesktopCommands::Remind { command } => print_reply(shell, &remind_call(command, now_ms())),
        DesktopCommands::Custom { command } => custom(shell, command),
        DesktopCommands::Keyboard => print_reply(shell, &IpcCall::new("keyboard", "status")),
        DesktopCommands::Gallery { close } => print_reply(
            shell,
            &IpcCall::new("gallery", if *close { "close" } else { "open" }),
        ),
    }
}

/// Relay one call and print the shell's reply.
fn print_reply(shell: &mut dyn Shell, call: &IpcCall) -> Result<()> {
    match shell.call(call) {
        Some(r) => println!("{r}"),
        None => println!("no responsive shell instance"),
    }
    Ok(())
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn panel_call(name: Option<&str>, close: bool) -> IpcCall {
    match (close, name) {
        (true, _) => IpcCall::new("panel", "close"),
        (false, Some(n)) => IpcCall::new("panel", "toggle").arg(n),
        (false, None) => IpcCall::new("panel", "status"),
    }
}

fn switch_call(target: &'static str, state: &SwitchCommands) -> IpcCall {
    IpcCall::new(
        target,
        match state {
            SwitchCommands::On => "on",
            SwitchCommands::Off => "off",
            SwitchCommands::Toggle => "toggle",
            SwitchCommands::Status => "status",
        },
    )
}

/// A reminder is handed to the shell as the wall-clock ms it fires at.
fn remind_call(command: &RemindCommands, now_ms: u64) -> IpcCall {
    match command {
        RemindCommands::Add { text, delay_ms } => IpcCall::new("reminders", "add")
            .arg(text)
            .arg(now_ms.saturating_add(*delay_ms).to_string()),
        RemindCommands::List => IpcCall::new("reminders", "list"),
        RemindCommands::Clear => IpcCall::new("reminders", "clear"),
    }
}

fn custom(shell: &mut dyn Shell, command: &CustomCommands) -> Result<()> {
    let call = match command {
        CustomCommands::Refresh { name } => IpcCall::new("custom", "refresh").arg(name),
        CustomCommands::Status { name } => IpcCall::new("custom", "status").arg(name),
    };
    match shell.call(&call) {
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

/// With no action: toggle the shell's power menu. With one: run it directly —
/// systemd verbs work without a shell, `lock` keeps its loud-failure path.
fn power(shell: &mut dyn Shell, action: Option<&PowerCommands>) -> Result<()> {
    let Some(action) = action else {
        return print_reply(shell, &IpcCall::new("power", "toggle"));
    };
    match action {
        PowerCommands::Lock => lock(shell, None),
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

/// The call that opens the picker for session `id`: a list of items, or a
/// free-text field.
fn picker_call(text_only: bool, id: &str, prompt: &str) -> IpcCall {
    IpcCall::new("launcher", if text_only { "inputText" } else { "select" })
        .arg(id)
        .arg(prompt)
}

/// dmenu mode. Items come in on stdin (one per line; `input` takes none),
/// go to the shell through a session file, and the choice comes back through
/// a result file the shell writes on every exit path — pick, cancel, close.
/// dmenu convention on cancel: exit 1, nothing on stdout.
fn select(shell: &mut dyn Shell, prompt: &str, text_only: bool) -> Result<()> {
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
    let id = format!("{}-{}", std::process::id(), now_ms());
    let items_path = dir.join(format!("select-{id}.json"));
    let result_path = dir.join(format!("select-{id}.result"));
    std::fs::write(
        &items_path,
        serde_json::json!({ "items": items }).to_string(),
    )
    .map_err(|e| VogixError::Config(format!("cannot write {}: {e}", items_path.display())))?;

    if shell.call(&picker_call(text_only, &id, prompt)).is_none() {
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
fn lock(shell: &mut dyn Shell, wait_secure: Option<f64>) -> Result<()> {
    let reply = shell.call(&IpcCall::new("lock", "lock")).ok_or_else(|| {
        VogixError::Config("cannot lock: no responsive vogix shell instance".to_string())
    })?;
    if reply.starts_with("refused") {
        return Err(VogixError::Config(format!("cannot lock: {reply}")));
    }
    if let Some(secs) = wait_secure {
        let deadline = Instant::now() + Duration::from_secs_f64(secs);
        loop {
            match shell.call(&IpcCall::new("lock", "status")).as_deref() {
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

fn lock_status(shell: &mut dyn Shell) -> Result<()> {
    match shell.call(&IpcCall::new("lock", "status")) {
        Some(state) => println!("{state}"),
        None => println!("unlocked (no shell instance)"),
    }
    Ok(())
}

/// Restart the shell — refused while locked: the WlSessionLock lives in the
/// shell process, so restarting it would drop the lock and expose the
/// session.
fn restart(shell: &mut dyn Shell) -> Result<()> {
    match shell.call(&IpcCall::new("lock", "status")).as_deref() {
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

fn notify(shell: &mut dyn Shell, command: &NotifyCommands) -> Result<()> {
    match command {
        NotifyCommands::Dismiss { all } => {
            let verb = if *all { "dismissAll" } else { "dismiss" };
            if shell.call(&IpcCall::new("notify", verb)).is_none() {
                debug!("desktop notify {verb}: no responsive shell instance");
            }
            Ok(())
        }
        NotifyCommands::Dnd { state } => dnd(shell, state),
        NotifyCommands::History { count } => history(*count),
    }
}

fn dnd(shell: &mut dyn Shell, state: &DndCommands) -> Result<()> {
    match state {
        DndCommands::On => {
            if shell.call(&IpcCall::new("notify", "dndOn")).is_none() {
                debug!("desktop notify dnd on: no responsive shell instance");
            }
        }
        DndCommands::Off => {
            if shell.call(&IpcCall::new("notify", "dndOff")).is_none() {
                debug!("desktop notify dnd off: no responsive shell instance");
            }
        }
        DndCommands::Toggle => match shell.call(&IpcCall::new("notify", "dndToggle")) {
            Some(now) => println!("dnd: {now}"),
            None => debug!("desktop notify dnd toggle: no responsive shell instance"),
        },
        DndCommands::Status => {
            // Prefer the live shell; fall back to the state file it persists,
            // so a TTY can still answer.
            let state = shell
                .call(&IpcCall::new("notify", "dndStatus"))
                .unwrap_or_else(|| {
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

fn background(shell: &mut dyn Shell, command: &crate::cli::BackgroundCommands) -> Result<()> {
    use crate::cli::BackgroundCommands as B;
    let call = match command {
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
            IpcCall::new("background", "set").arg(canonical.to_string_lossy())
        }
        B::Next => IpcCall::new("background", "next"),
        B::Clear => IpcCall::new("background", "clear"),
        B::Status => IpcCall::new("background", "status"),
    };
    print_reply(shell, &call)
}

fn osd(
    shell: &mut dyn Shell,
    kind: &str,
    value: Option<u8>,
    muted: bool,
    message: Option<&str>,
) -> Result<()> {
    let value = value.map(|v| v.min(100) as i32).unwrap_or(-1).to_string();
    // Transport function is `flash`: "show" is quickshell's own `ipc call
    // <target> show` introspection verb and never reaches a handler.
    let call = IpcCall::new("osd", "flash")
        .arg(kind)
        .arg(value)
        .arg(if muted { "true" } else { "false" })
        .arg(message.unwrap_or(""));
    if shell.call(&call).is_none() {
        debug!("desktop osd {kind}: no responsive shell instance");
    }
    Ok(())
}

/// Run one `qs ipc` call against the vogix shell instance, bounded. Returns
/// the trimmed stdout, or None when no responsive instance exists — qs is
/// not installed, no instance answers within 2 s, or qs reports a failure.
/// qs conflates its failures onto stdout with exit 0 ("Target not found.",
/// "No running instances…", "Not ready to accept queries yet"), so those
/// are normalized to None here rather than surfacing as shell replies.
fn qs_ipc(call: &IpcCall) -> Option<String> {
    let mut child = Command::new("qs")
        .args(["-c", "vogix", "ipc", "call", call.target, call.function])
        .args(&call.args)
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
fn status(shell: &mut dyn Shell) -> Result<()> {
    match shell.call(&IpcCall::new("bar", "status")) {
        Some(bar_state) => println!("shell: running\nbar: {bar_state}"),
        None => println!("shell: not running"),
    }
    Ok(())
}

fn bar(shell: &mut dyn Shell, command: &BarCommands) -> Result<()> {
    // Transport name is `unhide`: "show" is quickshell's own `ipc call
    // <target> show` introspection verb and never reaches a handler.
    let call = match command {
        BarCommands::Show { edge } => IpcCall::new("bar", "unhide").arg(edge),
        BarCommands::Hide { edge } => IpcCall::new("bar", "hide").arg(edge),
        BarCommands::Toggle { edge } => IpcCall::new("bar", "toggle").arg(edge),
        BarCommands::Status => IpcCall::new("bar", "status"),
        BarCommands::Geometry => IpcCall::new("bar", "geometry"),
    };
    match shell.call(&call) {
        Some(out) => {
            if !out.is_empty() {
                println!("{out}");
            }
        }
        None => debug!(
            "desktop bar {}: no responsive shell instance",
            call.function
        ),
    }
    Ok(())
}

/// Ask the running shell to re-read `theme.json`, `desktop.json` and the
/// theme's `backgrounds.json`. The store-symlink swap a theme switch
/// performs is invisible to Qt's file watcher, so the reload is an explicit
/// verb wired as the app's `reload_command`.
///
/// No shell instance — a TTY session, tests, the shell not enabled, qs not
/// installed — is SUCCESS by design: this runs on every theme switch for
/// every desktop user, and a missing shell must never fail the switch. The
/// call is bounded, so a wedged shell cannot hang the switch either.
fn reload(shell: &mut dyn Shell) -> Result<()> {
    if shell.call(&IpcCall::new("theme", "reload")).is_none() {
        debug!("desktop reload: no responsive shell instance — nothing to reload");
    }
    Ok(())
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

    // ── The verbs' transport: what each `vogix desktop` verb asks of the
    // shell, and that the shell (desktop/shell.qml) answers exactly that.

    /// Records every call a verb makes and answers it from `answer`: the
    /// verbs' own code, with the `qs` process replaced.
    struct Recorder<F: FnMut(&IpcCall) -> Option<String>> {
        calls: Vec<IpcCall>,
        answer: F,
    }

    impl<F: FnMut(&IpcCall) -> Option<String>> Shell for Recorder<F> {
        fn call(&mut self, call: &IpcCall) -> Option<String> {
            self.calls.push(call.clone());
            (self.answer)(call)
        }
    }

    /// A shell that answers every call, reporting the lock SECURE (so
    /// `lock --wait-secure` returns at once and `restart` is refused
    /// before it could reach systemctl).
    fn answer(call: &IpcCall) -> Option<String> {
        Some(match (call.target, call.function) {
            ("lock", "status") => "secure".to_string(),
            _ => "ok".to_string(),
        })
    }

    /// Parse `vogix desktop ARGV…` with the real CLI and run it against a
    /// recording shell.
    fn drive(
        argv: &[&str],
        answer: impl FnMut(&IpcCall) -> Option<String>,
    ) -> (Result<()>, Vec<IpcCall>) {
        use clap::Parser;
        let full = ["vogix", "desktop"].iter().chain(argv).copied();
        let cli = crate::cli::Cli::try_parse_from(full)
            .unwrap_or_else(|e| panic!("vogix desktop {argv:?}: {e}"));
        let crate::cli::Commands::Desktop { command } = cli.command else {
            unreachable!("parsed as `vogix desktop`")
        };
        let mut shell = Recorder {
            calls: Vec::new(),
            answer,
        };
        let result = run(&command, &mut shell);
        (result, shell.calls)
    }

    fn call(target: &'static str, function: &'static str, args: &[&str]) -> IpcCall {
        args.iter()
            .fold(IpcCall::new(target, function), |c, a| c.arg(*a))
    }

    /// Every verb that talks to the shell, and the calls it makes — the
    /// transport names that differ from the verb included (bar show →
    /// unhide, osd → flash, notify dismiss --all → dismissAll).
    fn relayed_verbs() -> Vec<(Vec<&'static str>, Vec<IpcCall>)> {
        vec![
            (vec!["reload"], vec![call("theme", "reload", &[])]),
            (vec!["status"], vec![call("bar", "status", &[])]),
            (vec!["meters"], vec![call("meters", "status", &[])]),
            (vec!["stats"], vec![call("stats", "status", &[])]),
            (vec!["privacy"], vec![call("privacy", "status", &[])]),
            (vec!["bar", "show"], vec![call("bar", "unhide", &["all"])]),
            (
                vec!["bar", "show", "top"],
                vec![call("bar", "unhide", &["top"])],
            ),
            (
                vec!["bar", "hide", "left"],
                vec![call("bar", "hide", &["left"])],
            ),
            (vec!["bar", "toggle"], vec![call("bar", "toggle", &["all"])]),
            (vec!["bar", "status"], vec![call("bar", "status", &[])]),
            (vec!["bar", "geometry"], vec![call("bar", "geometry", &[])]),
            (
                vec!["notify", "dismiss"],
                vec![call("notify", "dismiss", &[])],
            ),
            (
                vec!["notify", "dismiss", "--all"],
                vec![call("notify", "dismissAll", &[])],
            ),
            (
                vec!["notify", "dnd", "on"],
                vec![call("notify", "dndOn", &[])],
            ),
            (
                vec!["notify", "dnd", "off"],
                vec![call("notify", "dndOff", &[])],
            ),
            (
                vec!["notify", "dnd", "toggle"],
                vec![call("notify", "dndToggle", &[])],
            ),
            (
                vec!["notify", "dnd", "status"],
                vec![call("notify", "dndStatus", &[])],
            ),
            (vec!["lock"], vec![call("lock", "lock", &[])]),
            (
                vec!["lock", "--wait-secure", "1"],
                vec![call("lock", "lock", &[]), call("lock", "status", &[])],
            ),
            (vec!["lock", "status"], vec![call("lock", "status", &[])]),
            (
                vec!["background", "next"],
                vec![call("background", "next", &[])],
            ),
            (
                vec!["background", "clear"],
                vec![call("background", "clear", &[])],
            ),
            (
                vec!["background", "status"],
                vec![call("background", "status", &[])],
            ),
            (
                vec![
                    "osd",
                    "volume",
                    "--value",
                    "40",
                    "--muted",
                    "--message",
                    "Speakers",
                ],
                vec![call("osd", "flash", &["volume", "40", "true", "Speakers"])],
            ),
            (
                vec!["osd", "caps"],
                vec![call("osd", "flash", &["caps", "-1", "false", ""])],
            ),
            (vec!["launcher"], vec![call("launcher", "open", &["", ""])]),
            (
                vec!["launcher", "--mode", "calc", "--query", "2+2"],
                vec![call("launcher", "open", &["calc", "2+2"])],
            ),
            (vec!["menu"], vec![call("launcher", "menu", &[""])]),
            (
                vec!["menu", "--summon", "power"],
                vec![call("launcher", "menu", &["power"])],
            ),
            (vec!["power"], vec![call("power", "toggle", &[])]),
            (vec!["power", "lock"], vec![call("lock", "lock", &[])]),
            (vec!["panel"], vec![call("panel", "status", &[])]),
            (
                vec!["panel", "calendar"],
                vec![call("panel", "toggle", &["calendar"])],
            ),
            (vec!["panel", "--close"], vec![call("panel", "close", &[])]),
            (
                vec!["nightlight", "on"],
                vec![call("nightlight", "on", &[])],
            ),
            (
                vec!["nightlight", "off"],
                vec![call("nightlight", "off", &[])],
            ),
            (
                vec!["nightlight", "toggle"],
                vec![call("nightlight", "toggle", &[])],
            ),
            (
                vec!["nightlight", "status"],
                vec![call("nightlight", "status", &[])],
            ),
            (vec!["stay-awake", "on"], vec![call("stayawake", "on", &[])]),
            (
                vec!["stay-awake", "toggle"],
                vec![call("stayawake", "toggle", &[])],
            ),
            (vec!["remind", "list"], vec![call("reminders", "list", &[])]),
            (
                vec!["remind", "clear"],
                vec![call("reminders", "clear", &[])],
            ),
            (
                vec!["custom", "refresh", "updates"],
                vec![call("custom", "refresh", &["updates"])],
            ),
            (
                vec!["custom", "status", "updates"],
                vec![call("custom", "status", &["updates"])],
            ),
            (vec!["keyboard"], vec![call("keyboard", "status", &[])]),
            (vec!["gallery"], vec![call("gallery", "open", &[])]),
            (
                vec!["gallery", "--close"],
                vec![call("gallery", "close", &[])],
            ),
        ]
    }

    #[test]
    fn every_verb_makes_its_shell_calls() {
        for (argv, want) in relayed_verbs() {
            let (result, calls) = drive(&argv, answer);
            assert!(result.is_ok(), "vogix desktop {argv:?}: {result:?}");
            assert_eq!(calls, want, "vogix desktop {argv:?}");
        }
    }

    /// `remind add` hands the shell the text and the wall-clock ms the
    /// reminder fires at.
    #[test]
    fn remind_add_sends_its_firing_time() {
        let before = now_ms();
        let (result, calls) = drive(&["remind", "add", "Tea", "--in", "10m"], answer);
        let after = now_ms();
        assert!(result.is_ok());
        assert_eq!(calls.len(), 1, "{calls:?}");
        let (target, function) = (calls[0].target, calls[0].function);
        assert_eq!((target, function), ("reminders", "add"));
        assert_eq!(calls[0].args[0], "Tea");
        let at: u64 = calls[0].args[1].parse().expect("a ms timestamp");
        assert!((before + 600_000..=after + 600_000).contains(&at), "{at}");
    }

    /// `background set` hands the shell the file's canonical path, and
    /// refuses a file that does not exist without calling the shell.
    #[test]
    fn background_set_sends_the_canonical_path() {
        let dir = tempfile::tempdir().expect("tempdir");
        let image = dir.path().join("wall.png");
        std::fs::write(&image, b"png").expect("write");
        let spelled = dir.path().join(".").join("wall.png");
        let (result, calls) = drive(&["background", "set", &spelled.to_string_lossy()], answer);
        assert!(result.is_ok());
        let canonical = image.canonicalize().expect("canonical");
        assert_eq!(
            calls,
            [call("background", "set", &[&canonical.to_string_lossy()])]
        );

        let missing = dir.path().join("missing.png");
        let (result, calls) = drive(&["background", "set", &missing.to_string_lossy()], answer);
        assert!(result.is_err());
        assert!(calls.is_empty(), "{calls:?}");
    }

    /// `restart` asks for the lock state first and refuses while locked,
    /// before it would restart anything.
    #[test]
    fn restart_is_refused_while_locked() {
        for state in ["locked", "secure"] {
            let (result, calls) = drive(&["restart"], |_| Some(state.to_string()));
            assert!(result.is_err(), "{state}");
            assert_eq!(calls, [call("lock", "status", &[])]);
        }
    }

    /// A lock the shell refuses, or no shell at all, is a failure; a name
    /// the shell's custom table lacks is too.
    #[test]
    fn loud_verbs_fail_on_refusal_or_absence() {
        let (result, _) = drive(&["lock"], |_| Some("refused: no PAM service".to_string()));
        assert!(result.is_err());
        let (result, _) = drive(&["lock"], |_| None);
        assert!(result.is_err());
        let (result, _) = drive(&["custom", "status", "nope"], |_| {
            Some("unknown custom cell: nope".to_string())
        });
        assert!(result.is_err());
        // Every other verb tolerates a missing shell.
        for (argv, _) in relayed_verbs() {
            if argv[0] == "lock" || argv[..] == ["power", "lock"] {
                continue;
            }
            let (result, _) = drive(&argv, |_| None);
            assert!(result.is_ok(), "vogix desktop {argv:?} without a shell");
        }
    }

    /// The IPC surface desktop/shell.qml declares: target → function →
    /// parameter count, read from its `IpcHandler { target: …; function
    /// f(…) }` blocks.
    fn shell_ipc_surface()
    -> std::collections::BTreeMap<String, std::collections::BTreeMap<String, usize>> {
        const SHELL_QML: &str = include_str!("../../desktop/shell.qml");
        let code: String = SHELL_QML
            .lines()
            .map(|l| l.split("//").next().unwrap_or(""))
            .collect::<Vec<_>>()
            .join("\n");
        let mut surface = std::collections::BTreeMap::new();
        let mut rest = code.as_str();
        while let Some(at) = rest.find("IpcHandler {") {
            let body_start = at + "IpcHandler {".len();
            let mut depth = 1;
            let mut end = rest.len();
            for (i, c) in rest[body_start..].char_indices() {
                match c {
                    '{' => depth += 1,
                    '}' => {
                        depth -= 1;
                        if depth == 0 {
                            end = body_start + i;
                            break;
                        }
                    }
                    _ => {}
                }
            }
            let body = &rest[body_start..end];
            let target = body
                .lines()
                .find_map(|l| l.trim().strip_prefix("target: \""))
                .and_then(|t| t.strip_suffix('"'))
                .expect("every IpcHandler names its target")
                .to_string();
            let mut functions = std::collections::BTreeMap::new();
            for piece in body.split("function ").skip(1) {
                let (name, after) = piece.split_once('(').expect("function NAME(");
                let params = after.split_once(')').expect("…)").0;
                let count = params.split(',').filter(|p| !p.trim().is_empty()).count();
                functions.insert(name.trim().to_string(), count);
            }
            assert!(
                surface.insert(target.clone(), functions).is_none(),
                "two IpcHandlers for {target}"
            );
            rest = &rest[end..];
        }
        surface
    }

    /// Every call a verb makes names a function the shell declares, with
    /// the shell's parameter count, and never `show` (quickshell's own
    /// `ipc call <target> show` introspection, which no handler receives);
    /// and every target the shell declares is reached by some verb, so the
    /// IPC surface and the verbs mirror each other.
    #[test]
    fn the_verbs_and_the_shell_ipc_surface_match() {
        let surface = shell_ipc_surface();
        // What the verbs actually send, not what the table above expects.
        let mut calls: Vec<IpcCall> = relayed_verbs()
            .into_iter()
            .flat_map(|(argv, _)| drive(&argv, answer).1)
            .collect();
        calls.push(remind_call(
            &RemindCommands::Add {
                text: "t".into(),
                delay_ms: 1,
            },
            0,
        ));
        calls.push(picker_call(false, "id", "prompt"));
        calls.push(picker_call(true, "id", "prompt"));
        for c in &calls {
            assert_ne!(c.function, "show", "{c:?}");
            let functions = surface
                .get(c.target)
                .unwrap_or_else(|| panic!("shell.qml has no IpcHandler for {c:?}"));
            let params = functions
                .get(c.function)
                .unwrap_or_else(|| panic!("shell.qml's {} handler has no {c:?}", c.target));
            assert_eq!(*params, c.args.len(), "{c:?}");
        }
        for target in surface.keys() {
            assert!(
                calls.iter().any(|c| c.target == target),
                "no vogix desktop verb reaches the shell's `{target}` IPC target"
            );
        }
    }
}

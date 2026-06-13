//! `vogix input` command handlers.
//!
//! `check` loads the input schema (the keybinding ontology, rendered from
//! `defaults.nix`), derives the praxis mode graph from it, and validates the
//! *loaded* graph against its structural axioms (no dead states, root
//! reachable). The engine *dynamics* axioms — quasimode reverts to root, exit
//! always reaches root (the no-stuck guarantee) — are not re-checked per
//! config here: praxis proves them as universal theorems over arbitrary mode
//! graphs (`pr4xis_domains::applied::hmi::input::engine` proptests), so they
//! hold for any graph this loader can produce.
//!
//! `run` is the production entry point: it loads the same schema, validates
//! the loaded graph (so we fail fast at startup rather than partway through a
//! key event), then takes over the keyboard via the device layer.

use crate::errors::{Result, VogixError};
use crate::input::device;
use crate::input::health;
use crate::input::schema::Schema;
use std::path::PathBuf;

fn load_schema(config: Option<&str>) -> Result<Schema> {
    match config {
        Some(path) => Schema::from_file(&PathBuf::from(path)),
        None => Schema::load(),
    }
}

/// Load the schema, derive the mode graph, and validate graph + engine axioms.
pub fn handle_input_check(config: Option<&str>) -> Result<()> {
    let schema = load_schema(config)?;
    let graph = schema.build_mode_graph();

    println!("vogix input — ontology check");
    println!(
        "  source: {}",
        config
            .map(String::from)
            .unwrap_or_else(|| Schema::default_path().display().to_string())
    );
    println!("  modes: {} (root: {})", graph.modes.len(), graph.root.0);
    println!("  transitions: {}", graph.transitions.len());
    println!("  tap-hold: {}ms", schema.tap_hold_ms());
    if let Some(target) = schema.caps_target() {
        println!("  CapsLock enters: {target}");
    }

    // Structural axioms on the derived graph (NoDeadStates / RootReachable / …).
    // The engine dynamics axioms (quasimode reverts, exit reaches root) are NOT
    // re-checked here — praxis proves them over arbitrary graphs, so they hold
    // for any graph this loader produces; see the module docs.
    let graph_failures = graph.validate();
    // Bindings that won't parse are silently dropped at router-build time, so
    // surface them here too — a mistyped key is a real, invisible config bug.
    let unparseable = schema.unparseable_bindings();

    // Remap-set axioms (praxis): the loaded paradigm's remaps must be injective,
    // and a macOS paradigm must cover the essential shortcuts (C/V/X/Z/S/A).
    let remap_failures = remap_axiom_failures(&schema);
    println!(
        "  paradigm: {} ({} remaps)",
        schema.paradigm(),
        schema.remap_set().remaps.len()
    );

    for f in &graph_failures {
        println!("  graph axiom FAILED: {f}");
    }
    for u in &unparseable {
        println!("  binding DROPPED (unparseable, won't work): {u}");
    }
    for f in &remap_failures {
        println!("  remap axiom FAILED: {f}");
    }
    if graph_failures.is_empty() && unparseable.is_empty() && remap_failures.is_empty() {
        println!("  graph axioms: OK");
        println!("  bindings: all parse");
        println!("  remap axioms: OK");
        println!("  engine dynamics (no-stuck): proven upstream in praxis");
        println!("all input axioms hold");
        Ok(())
    } else {
        Err(VogixError::Config(format!(
            "{} graph axiom(s) failed, {} binding(s) unparseable, {} remap axiom(s) failed",
            graph_failures.len(),
            unparseable.len(),
            remap_failures.len(),
        )))
    }
}

/// `vogix input help` — show the resolved schema's keybindings.
///
/// Materialized from the single resolved [`Schema`] (the selected paradigm's nav
/// merged with the user overlay), so it reflects whatever paradigm is active —
/// the engine-side replacement for the build-time Nix `vogix-modes-*` scripts.
/// The paradigm is loaded once and every view (dispatch, help, Hyprland fallback)
/// materialises from it; nothing re-encodes the keymap.
pub fn handle_input_help(print: bool, config: Option<&str>) -> Result<()> {
    let schema = load_schema(config)?;
    let text = format_help(&schema);
    if print {
        println!("{text}");
        return Ok(());
    }
    // Searchable walker menu if present, else a desktop notification.
    if pipe_to("walker", &["--dmenu", "-p", "Keybindings"], &text).is_err() {
        let _ = std::process::Command::new("notify-send")
            .args(["-t", "10000", "Keybindings", &text])
            .status();
    }
    Ok(())
}

/// Format the root mode's described keybindings as sorted `KEY  description`
/// columns — mirrors the old Nix help generator's output.
fn format_help(schema: &Schema) -> String {
    let mode = schema
        .modes
        .get(&schema.mode_graph.root)
        .or_else(|| schema.modes.get("app"));
    let mut lines: Vec<String> = mode
        .map(|m| {
            m.bindings
                .values()
                .filter_map(|b| {
                    let desc = b.description.as_deref().filter(|d| !d.is_empty())?;
                    Some(format!("{:<20} {desc}", format_key(&b.key)))
                })
                .collect()
        })
        .unwrap_or_default();
    lines.sort();
    lines.join("\n")
}

/// Display form of a chord: `super + `/`modKey + ` → `Super+`, then uppercased.
fn format_key(key: &str) -> String {
    key.replace("modKey + ", "Super+")
        .replace("super + ", "Super+")
        .to_uppercase()
}

/// Pipe `input` to a spawned `cmd args` via stdin (errors if the binary is absent).
fn pipe_to(cmd: &str, args: &[&str], input: &str) -> std::io::Result<()> {
    use std::io::Write;
    use std::process::{Command, Stdio};
    let mut child = Command::new(cmd).args(args).stdin(Stdio::piped()).spawn()?;
    child
        .stdin
        .take()
        .expect("piped stdin")
        .write_all(input.as_bytes())?;
    child.wait()?;
    Ok(())
}

/// Verify the loaded paradigm's remap set against the praxis keybinding axioms.
/// Returns the names of any that fail (empty = all hold).
fn remap_axiom_failures(schema: &Schema) -> Vec<String> {
    use pr4xis::ontology::Axiom;
    use pr4xis_domains::applied::hmi::input::keybindings::{MacosRemapComplete, RemapInjective};

    let remaps = schema.remap_set();
    let mut failures = Vec::new();
    let injective = RemapInjective {
        remaps: remaps.clone(),
    };
    if injective.verify().is_err() {
        failures.push("RemapInjective".to_string());
    }
    // Completeness is only required of the macOS paradigm (the one that claims it).
    if schema.paradigm() == "macos" {
        let complete = MacosRemapComplete { remaps };
        if complete.verify().is_err() {
            failures.push("MacosRemapComplete".to_string());
        }
    }
    failures
}

/// Load the schema, validate the loaded graph fail-closed, then hand off to
/// the device loop.
///
/// We refuse to grab the keyboard if the loaded graph fails its structural
/// axioms (dead state / unreachable root) — running the engine over such a
/// graph is exactly the "stuck in a mode" foot-gun this module makes
/// unrepresentable. The engine *dynamics* axioms hold for any valid graph
/// (proven in praxis), so a structurally-valid graph is sufficient.
pub fn handle_input_run(config: Option<&str>) -> Result<()> {
    let schema = load_schema(config)?;
    let graph = schema.build_mode_graph();

    // Fail-closed startup gate: structural axioms must hold on the loaded graph.
    let graph_failures = graph.validate();
    if !graph_failures.is_empty() {
        return Err(VogixError::Config(format!(
            "input schema rejected by {} graph axiom(s): {}",
            graph_failures.len(),
            graph_failures.join("; ")
        )));
    }

    // A binding that won't parse is dropped silently by the router; warn so it's
    // visible in the journal rather than failing the whole keyboard over a typo.
    for u in &schema.unparseable_bindings() {
        log::warn!("vogix input: binding will be dropped (unparseable): {u}");
    }

    log::info!(
        "vogix input — engine starting: {} modes, {} transitions, tap-hold {}ms",
        graph.modes.len(),
        graph.transitions.len(),
        schema.tap_hold_ms(),
    );
    device::run(schema)
}

/// `vogix input doctor` — read-only diagnostics. Renders the health snapshot the
/// running engine writes: which keyboards are grabbed, each one's event flow, a
/// SILENT flag for a device gone quiet (the tell that localises a flaky keyboard
/// to the hardware, not the engine), the stuck-key count, the mode, and the
/// snapshot age. Never grabs the keyboard; the snapshot carries NO key identity,
/// so this never logs keystrokes. `--watch` repaints continuously.
pub fn handle_input_doctor(watch: bool) -> Result<()> {
    use std::io::Write;
    loop {
        if watch {
            print!("\x1b[2J\x1b[H"); // clear + cursor home
        }
        match health::read_snapshot() {
            Some(snap) => print_health(&snap),
            None => println!(
                "no health snapshot — is `vogix input run` active? \
                 (it writes ~/.local/state/vogix/input-health.json about once a second)"
            ),
        }
        if !watch {
            return Ok(());
        }
        let _ = std::io::stdout().flush();
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
}

fn print_health(s: &health::HealthSnapshot) {
    println!(
        "vogix input engine — pid {}  mode {}  up {}s",
        s.pid,
        s.mode,
        s.uptime_ms / 1000
    );
    if s.stuck_count > 0 {
        println!(
            "  ⚠ {} stuck key(s) (oldest held {}ms)",
            s.stuck_count, s.stuck_oldest_ms
        );
    }
    if s.devices.is_empty() {
        println!("  (no keyboards grabbed)");
    }
    for d in &s.devices {
        // 3s with no event from a grabbed keyboard is the "went quiet" signal.
        let silent = if d.silent_ms >= 3000 {
            "  <-- SILENT"
        } else {
            ""
        };
        // Show every tally so `in` balances against the rest (in = emit +
        // dispatch + keyword + mode + swallow); omitting keyword/mode made the
        // line look like a counting bug whenever a mode flip or border paint fired.
        println!(
            "  {:<34} {:04x}:{:04x}  in={} emit={} dispatch={} keyword={} mode={} swallow={} last-seen={}s ago{}",
            d.name,
            d.vendor,
            d.product,
            d.counters.events_in,
            d.counters.emitted,
            d.counters.dispatched,
            d.counters.keyword,
            d.counters.mode_changes,
            d.counters.swallowed,
            d.silent_ms / 1000,
            silent,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::schema::Schema;

    #[test]
    fn help_materializes_from_the_resolved_schema() {
        // New format: overlay + paradigm name; the engine resolves the nav, and
        // the help is materialized from that single resolved schema.
        let json = r#"{
            "keybindings": { "paradigm": "vogix" },
            "modes": { "app": { "bindings": {
                "launcher": { "key": "super + space", "action": "exec, walker", "description": "Launcher" }
            } } }
        }"#;
        let help = format_help(&Schema::from_json(json).expect("resolve"));
        assert!(help.contains("Launcher"), "overlay binding shown: {help}");
        assert!(
            help.to_uppercase().contains("SUPER+H"),
            "resolved nav binding shown: {help}"
        );
    }

    #[test]
    fn format_key_collapses_super_and_uppercases() {
        assert_eq!(format_key("super + shift + h"), "SUPER+SHIFT + H");
        assert_eq!(format_key("modKey + tab"), "SUPER+TAB");
    }
}

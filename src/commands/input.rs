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
    for f in &graph_failures {
        println!("  graph axiom FAILED: {f}");
    }
    for u in &unparseable {
        println!("  binding DROPPED (unparseable, won't work): {u}");
    }
    if graph_failures.is_empty() && unparseable.is_empty() {
        println!("  graph axioms: OK");
        println!("  bindings: all parse");
        println!("  engine dynamics (no-stuck): proven upstream in praxis");
        println!("all input axioms hold");
        Ok(())
    } else {
        Err(VogixError::Config(format!(
            "{} input graph axiom(s) failed, {} binding(s) unparseable",
            graph_failures.len(),
            unparseable.len()
        )))
    }
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

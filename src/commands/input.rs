//! `vogix input` command handlers.
//!
//! `check` loads the input schema (the keybinding ontology, rendered from
//! `defaults.nix`), derives the praxis mode graph from it, and validates it
//! against the axioms — both the structural praxis axioms on the graph (no dead
//! states, root reachable) and the runtime invariants on the engine (quasimode
//! reverts, exit always reaches root). It is the self-test of the loaded
//! "schema": if these pass, the "stuck in a mode" class of bug cannot occur.

use crate::errors::{Result, VogixError};
use crate::input::engine::axioms;
use crate::input::schema::Schema;
use std::path::PathBuf;

/// Load the schema, derive the mode graph, and validate graph + engine axioms.
pub fn handle_input_check(config: Option<&str>) -> Result<()> {
    let schema = match config {
        Some(path) => Schema::from_file(&PathBuf::from(path))?,
        None => Schema::load()?,
    };
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

    let mut failures: Vec<String> = Vec::new();

    // Structural axioms on the derived graph (NoDeadStates / RootReachable / …).
    let graph_failures = graph.validate();
    if graph_failures.is_empty() {
        println!("  graph axioms: OK");
    } else {
        for f in &graph_failures {
            println!("  graph axiom FAILED: {f}");
        }
        failures.extend(graph_failures);
    }

    // Runtime invariants on the engine over this graph (no-stuck guarantee).
    for axiom in axioms(graph) {
        match axiom.verify() {
            Ok(_) => {
                println!("  engine axiom: OK — {}", axiom.description().as_str());
            }
            Err(_) => {
                println!("  engine axiom FAILED — {}", axiom.description().as_str());
                failures.push(axiom.description().as_str().to_string());
            }
        }
    }

    if failures.is_empty() {
        println!("all input axioms hold");
        Ok(())
    } else {
        Err(VogixError::Config(format!(
            "{} input axiom(s) failed",
            failures.len()
        )))
    }
}

//! The loaded input schema — vogix's keybinding *ontology instance*, read as
//! data rather than compiled in.
//!
//! Just as praxis loads a linguistic ontology (WordNet) at runtime instead of
//! hard-coding English, vogix loads its interaction ontology — the mode graph
//! and the per-mode bindings — from a config file and *interprets* it. The
//! engine ([`super::engine`]) and the device loop stay generic; this module is
//! the bridge from the authored config (`defaults.nix` →
//! `programs.vogix.keybindings`, rendered to JSON by the NixOS module) to the
//! praxis [`ModeGraph`] and the runtime binding tables.
//!
//! The JSON mirrors the Nix attrset 1:1, so the source of truth stays in one
//! place. Nothing here decides *which* modes or keys exist — that is all data.

// Several loaded fields (bindings' key/exitAfter/repeat, remaps, modKey, …) are
// consumed by the device loop in the next 2b step; allow until that lands.
#![allow(dead_code)]

use crate::config::Config;
use crate::errors::{Result, VogixError};
use pr4xis_domains::applied::hmi::input::modes::{ModeGraph, ModeId, ModeProperties};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// The whole loaded schema (mirrors `programs.vogix.keybindings` + `modeGraph`).
#[derive(Debug, Clone, Deserialize)]
pub struct Schema {
    #[serde(rename = "modeGraph")]
    pub mode_graph: ModeGraphSpec,
    pub modes: HashMap<String, ModeSpec>,
    #[serde(default)]
    pub keybindings: Keybindings,
    // `defaults.nix` names this `_superCtrlRemaps`; accept both.
    #[serde(default, rename = "superCtrlRemaps", alias = "_superCtrlRemaps")]
    pub super_ctrl_remaps: HashMap<String, RemapSpec>,
}

/// The mode topology: which modes exist and their parent/kind.
#[derive(Debug, Clone, Deserialize)]
pub struct ModeGraphSpec {
    pub root: String,
    pub modes: HashMap<String, ModeNode>,
}

/// One node in the mode topology.
#[derive(Debug, Clone, Deserialize)]
pub struct ModeNode {
    /// Exit target (`Esc`/release goes here). `None` for the root.
    #[serde(default)]
    pub parent: Option<String>,
    /// `"normal"` | `"submap"` | `"passthrough"`. `submap` = catchall (the
    /// engine owns unbound keys); others pass unbound keys through.
    #[serde(rename = "type", default)]
    pub kind: Option<String>,
}

/// A mode's contextual bindings.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct ModeSpec {
    #[serde(default)]
    pub enter: Option<String>,
    #[serde(default)]
    pub exit: Option<String>,
    #[serde(default)]
    pub bindings: HashMap<String, Binding>,
}

/// One binding: a key chord → a Hyprland action, with modal flags.
#[derive(Debug, Clone, Deserialize)]
pub struct Binding {
    /// Key chord, e.g. `"h"`, `"left"`, `"super + 1"`, `"shift + 0"`, `"F12"`.
    pub key: String,
    /// Hyprland action, e.g. `"movefocus, l"`, `"submap, move"`, `"exec, $TERMINAL"`.
    pub action: String,
    /// Return to root after running the action (one-shot commands).
    #[serde(default, rename = "exitAfter")]
    pub exit_after: bool,
    /// Re-run the action on key auto-repeat (focus/move/resize).
    #[serde(default)]
    pub repeat: bool,
    #[serde(default)]
    pub description: Option<String>,
}

/// Input-layer settings (mod key + CapsLock layer params).
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Keybindings {
    #[serde(default, rename = "modKey")]
    pub mod_key: Option<String>,
    #[serde(default)]
    pub layers: HashMap<String, Layer>,
}

/// A CapsLock-style dual-role layer (we read its tap/hold timing + hold action).
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Layer {
    #[serde(default)]
    pub hold: Option<String>,
    #[serde(default, rename = "tapHoldMs")]
    pub tap_hold_ms: Option<u64>,
    /// The keysym the hold emits (e.g. `"f23"`); a root-mode binding for this
    /// key declares which mode the hold enters.
    #[serde(default, rename = "holdAction")]
    pub hold_action: Option<String>,
}

/// A Super→Ctrl style remap (`from`/`to` are `"mod + key"` strings).
#[derive(Debug, Clone, Deserialize)]
pub struct RemapSpec {
    pub from: String,
    pub to: String,
}

/// A parsed Hyprland action: either a mode switch or a dispatch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActionKind {
    /// `submap, X` — a mode transition (`X == "reset"` means the root).
    Submap(String),
    /// Anything else — a Hyprland dispatcher invocation (`"movefocus, l"`, …).
    Dispatch(String),
}

/// Classify a Hyprland action string.
pub fn parse_action(action: &str) -> ActionKind {
    let t = action.trim();
    if let Some(rest) = t.strip_prefix("submap") {
        ActionKind::Submap(rest.trim_start_matches(',').trim().to_string())
    } else {
        ActionKind::Dispatch(t.to_string())
    }
}

/// Default tap↔hold threshold if the schema doesn't specify one.
pub const DEFAULT_TAP_HOLD_MS: u64 = 250;

impl Schema {
    /// Load the schema from the standard location
    /// (`~/.local/state/vogix/input.json`, rendered by the NixOS module).
    pub fn load() -> Result<Self> {
        Self::from_file(&Self::default_path())
    }

    /// The standard schema path.
    pub fn default_path() -> PathBuf {
        Config::state_dir().join("input.json")
    }

    /// Load from a specific path.
    pub fn from_file(path: &Path) -> Result<Self> {
        let text = std::fs::read_to_string(path).map_err(|e| {
            VogixError::Config(format!(
                "cannot read input schema at {}: {e}",
                path.display()
            ))
        })?;
        Self::from_json(&text)
    }

    /// Parse from a JSON string.
    pub fn from_json(text: &str) -> Result<Self> {
        serde_json::from_str(text)
            .map_err(|e| VogixError::Config(format!("input schema parse error: {e}")))
    }

    /// The CapsLock tap↔hold threshold (ms), from the dual-role layer.
    pub fn tap_hold_ms(&self) -> u64 {
        self.keybindings
            .layers
            .values()
            .find(|l| l.hold.as_deref() == Some("capslock"))
            .and_then(|l| l.tap_hold_ms)
            .unwrap_or(DEFAULT_TAP_HOLD_MS)
    }

    /// The mode CapsLock enters.
    ///
    /// Derived through the actual config linkage: the `capslock` layer's
    /// `holdAction` keysym (e.g. `"f23"`) is bound in the root mode to a
    /// `submap, X` action, and `X` is the target. This is unambiguous even when
    /// several modes are `submap` children of root (`move`/`resize` are too).
    pub fn caps_target(&self) -> Option<String> {
        let layer = self
            .keybindings
            .layers
            .values()
            .find(|l| l.hold.as_deref() == Some("capslock"))?;
        let hold_key = layer.hold_action.as_ref()?;
        let root_mode = self.modes.get(&self.mode_graph.root)?;
        root_mode
            .bindings
            .values()
            .find(|b| b.key.eq_ignore_ascii_case(hold_key))
            .and_then(|b| match parse_action(&b.action) {
                ActionKind::Submap(target) => Some(target),
                ActionKind::Dispatch(_) => None,
            })
    }

    /// Derive the praxis [`ModeGraph`] from the loaded topology and bindings.
    ///
    /// Edges are *derived*, not authored separately: a mode's `parent` is its
    /// exit edge; root → its direct children are the enter edges; and every
    /// `submap, X` binding is an enter/switch edge `M → X`. So the graph the
    /// engine validates against is exactly what the config describes.
    pub fn build_mode_graph(&self) -> ModeGraph {
        let root = ModeId::new(&self.mode_graph.root);
        let mut g = ModeGraph::new(root.clone());

        // Modes (root is already present from `ModeGraph::new`).
        for (name, node) in &self.mode_graph.modes {
            if name == &self.mode_graph.root {
                continue;
            }
            g.add_mode(
                ModeId::new(name),
                ModeProperties {
                    catchall: node.kind.as_deref() == Some("submap"),
                    parent: node.parent.clone().map(ModeId::new),
                    depth: self.depth_of(name),
                },
            );
        }

        // Exit edges (M → parent) and enter edges (root → direct child).
        for (name, node) in &self.mode_graph.modes {
            if let Some(parent) = &node.parent {
                g.add_transition(ModeId::new(name), ModeId::new(parent));
                if parent == &self.mode_graph.root {
                    g.add_transition(root.clone(), ModeId::new(name));
                }
            }
        }

        // Enter/switch edges from `submap, X` bindings.
        for (name, mode) in &self.modes {
            for b in mode.bindings.values() {
                if let ActionKind::Submap(target) = parse_action(&b.action) {
                    let to = if target == "reset" {
                        self.mode_graph.root.clone()
                    } else {
                        target
                    };
                    g.add_transition(ModeId::new(name), ModeId::new(to));
                }
            }
        }

        g
    }

    /// Depth of a mode = length of its parent chain to root (root = 0).
    fn depth_of(&self, name: &str) -> u8 {
        let mut depth = 0u8;
        let mut cur = name.to_string();
        while let Some(node) = self.mode_graph.modes.get(&cur) {
            match &node.parent {
                Some(parent) if parent != &cur && depth < 20 => {
                    depth += 1;
                    cur = parent.clone();
                }
                _ => break,
            }
        }
        depth
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A trimmed schema in the exact shape `defaults.nix` renders to.
    const FIXTURE: &str = r#"{
      "modeGraph": {
        "root": "app",
        "modes": {
          "app":     { "parent": null,      "type": "normal" },
          "desktop": { "parent": "app",     "type": "submap" },
          "move":    { "parent": "app",     "type": "submap" },
          "resize":  { "parent": "app",     "type": "submap" },
          "console": { "parent": "app",     "type": "passthrough" }
        }
      },
      "keybindings": {
        "modKey": "super",
        "layers": {
          "desktopToggle": { "hold": "capslock", "tapHoldMs": 250, "holdAction": "f23" }
        }
      },
      "superCtrlRemaps": {
        "copy": { "from": "super + c", "to": "ctrl + c" }
      },
      "modes": {
        "app": { "exit": "escape", "bindings": {
          "ws1": { "key": "super + 1", "action": "workspace, 1" },
          "enterDesktopHold": { "key": "F23", "action": "submap, desktop" },
          "console": { "key": "F12", "action": "exec, toggle-console" }
        }},
        "desktop": { "exit": "escape", "bindings": {
          "focusLeft": { "key": "h", "action": "movefocus, l", "repeat": true },
          "enterMove": { "key": "m", "action": "submap, move" },
          "enterResize": { "key": "r", "action": "submap, resize" },
          "close": { "key": "q", "action": "killactive,", "exitAfter": true },
          "exitToggle": { "key": "F24", "action": "submap, reset" }
        }},
        "move": { "exit": "escape", "bindings": {
          "moveLeft": { "key": "h", "action": "movewindow, l", "repeat": true },
          "toResize": { "key": "r", "action": "submap, resize" },
          "exitToggle": { "key": "F24", "action": "submap, reset" }
        }},
        "resize": { "exit": "escape", "bindings": {
          "resizeLeft": { "key": "h", "action": "resizeactive, -40 0", "repeat": true },
          "toMove": { "key": "m", "action": "submap, move" },
          "exitToggle": { "key": "F24", "action": "submap, reset" }
        }},
        "console": { "bindings": {
          "exitConsole": { "key": "F12", "action": "exec, close-console" }
        }}
      }
    }"#;

    fn schema() -> Schema {
        Schema::from_json(FIXTURE).expect("fixture parses")
    }

    #[test]
    fn parses_the_nix_shape() {
        let s = schema();
        assert_eq!(s.mode_graph.root, "app");
        assert_eq!(s.mode_graph.modes.len(), 5);
        assert_eq!(s.tap_hold_ms(), 250);
        assert_eq!(s.caps_target().as_deref(), Some("desktop"));
        assert_eq!(s.super_ctrl_remaps.len(), 1);
    }

    #[test]
    fn derived_graph_satisfies_praxis_axioms() {
        let g = schema().build_mode_graph();
        let failures = g.validate();
        assert!(
            failures.is_empty(),
            "derived graph must validate: {failures:?}"
        );
    }

    #[test]
    fn submap_bindings_become_transitions() {
        let g = schema().build_mode_graph();
        // enter sub-modes from desktop (from the enterMove/enterResize bindings)
        assert!(g.is_valid_transition(&ModeId::new("desktop"), &ModeId::new("move")));
        assert!(g.is_valid_transition(&ModeId::new("desktop"), &ModeId::new("resize")));
        // switch between sub-modes (toResize / toMove bindings)
        assert!(g.is_valid_transition(&ModeId::new("move"), &ModeId::new("resize")));
        assert!(g.is_valid_transition(&ModeId::new("resize"), &ModeId::new("move")));
        // caps enters desktop (root → direct catchall child)
        assert!(g.is_valid_transition(&ModeId::new("app"), &ModeId::new("desktop")));
        // exit edges (parent): sub-modes return to app, not desktop, per defaults.nix
        assert!(g.is_valid_transition(&ModeId::new("move"), &ModeId::new("app")));
    }

    #[test]
    fn move_parent_is_app_not_desktop() {
        // The discrepancy that hard-coding got wrong: defaults.nix sets
        // move/resize parent = app. The loaded config is the source of truth.
        let g = schema().build_mode_graph();
        assert_eq!(
            g.modes[&ModeId::new("move")].parent,
            Some(ModeId::new("app"))
        );
        assert_eq!(g.modes[&ModeId::new("move")].depth, 1);
    }

    #[test]
    fn action_parser_distinguishes_submap_from_dispatch() {
        assert_eq!(
            parse_action("submap, move"),
            ActionKind::Submap("move".into())
        );
        assert_eq!(
            parse_action("submap, reset"),
            ActionKind::Submap("reset".into())
        );
        assert_eq!(
            parse_action("movefocus, l"),
            ActionKind::Dispatch("movefocus, l".into())
        );
        assert_eq!(
            parse_action("exec, $TERMINAL"),
            ActionKind::Dispatch("exec, $TERMINAL".into())
        );
    }
}

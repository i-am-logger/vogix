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

// The schema mirrors the Nix keybinding attrset 1:1 (see the module docs above),
// so a few fields are authored + serialized for the NixOS generator layer but not
// read back by the Rust engine (which derives them from the praxis ontology).
// Those carry a targeted `#[allow(dead_code)]` at their definition rather than a
// blanket allow over the whole module.

use super::devfilter::DeviceFilter;
use crate::config::Config;
use crate::errors::{Result, VogixError};
use pr4xis_domains::applied::hmi::input::keybindings::{
    Key, KeyCombo, Modifier as PxMod, RemapSet, macos_remap,
};
use pr4xis_domains::applied::hmi::input::modes::{ModeGraph, ModeId, ModeProperties};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// The whole loaded schema (mirrors `programs.vogix.keybindings` + `modeGraph`).
#[derive(Debug, Clone, Deserialize)]
pub struct Schema {
    #[serde(rename = "modeGraph", default)]
    pub mode_graph: ModeGraphSpec,
    pub modes: HashMap<String, ModeSpec>,
    #[serde(default)]
    pub keybindings: Keybindings,
    /// Hyprland window classes that are terminals. When one is focused, the
    /// Super→Ctrl remap is context-adjusted (copy/paste → Ctrl+Shift+C/V; other
    /// remaps suppressed) so Super+C can't fire Ctrl+C=SIGINT into the shell.
    #[serde(default, rename = "terminalClasses")]
    pub terminal_classes: Vec<String>,
    /// Per-mode border colours (theme-derived, Hyprland `rgb(...)` form). Drive
    /// the mode-visibility surface: on a mode change the engine paints the active
    /// window border so the user can always SEE which mode is active (the cure
    /// for mode error — Norman 1981).
    #[serde(default, rename = "modeColors")]
    pub mode_colors: HashMap<String, ModeColor>,
    /// Device-grab policy: which evdev devices the engine may OWN. Optional;
    /// its excludes are MERGED on top of the safe baseline so the YubiKey / audio
    /// HID are dropped even with no config. See [`Schema::device_filter`].
    #[serde(default, rename = "deviceFilter")]
    pub device_filter: Option<DeviceFilterSpec>,
}

/// A mode's border colours (Hyprland `rgb(RRGGBB)` strings).
#[derive(Debug, Clone, Default, Deserialize)]
pub struct ModeColor {
    #[serde(default)]
    pub active: String,
    #[serde(default)]
    pub inactive: String,
}

/// Optional device-grab policy from the schema (`deviceFilter`). Its excludes
/// EXTEND the baked-in defaults ([`DeviceFilter::default`]) rather than replacing
/// them — adding a vendor cannot accidentally re-enable the YubiKey grab.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct DeviceFilterSpec {
    #[serde(default, rename = "excludeVendors")]
    pub exclude_vendors: Vec<u16>,
    #[serde(default, rename = "excludeNameSubstrings")]
    pub exclude_name_substrings: Vec<String>,
}

impl Schema {
    /// The effective device-grab filter: the baked-in safe baseline
    /// ([`DeviceFilter::default`]) PLUS any schema-provided excludes (merge, not
    /// replace — so a user adding a vendor keeps the Yubico/audio defaults).
    pub fn device_filter(&self) -> DeviceFilter {
        let mut f = DeviceFilter::default();
        if let Some(spec) = &self.device_filter {
            for v in &spec.exclude_vendors {
                if !f.exclude_vendors.contains(v) {
                    f.exclude_vendors.push(*v);
                }
            }
            for s in &spec.exclude_name_substrings {
                if !f.exclude_name_substrings.contains(s) {
                    f.exclude_name_substrings.push(s.clone());
                }
            }
        }
        f
    }
}

/// The mode topology: which modes exist and their parent/kind.
#[derive(Debug, Clone, Default, Deserialize)]
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
    // Schema-only: the engine enters modes via bindings' `entersMode`.
    #[serde(default)]
    #[allow(dead_code)]
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
    /// Human-facing label, mirrored from the Nix attrset; not read by the engine.
    #[serde(default)]
    #[allow(dead_code)]
    pub description: Option<String>,
}

/// Input-layer settings (mod key + CapsLock layer params).
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Keybindings {
    // Consumed by the Nix generator (`modKey`); the Rust engine derives the
    // Super modifier from the praxis ontology, so it is unread here.
    #[serde(default, rename = "modKey")]
    #[allow(dead_code)]
    pub mod_key: Option<String>,
    #[serde(default)]
    pub layers: HashMap<String, Layer>,
    /// Interaction-paradigm preset that supplies the Super-modifier remap set
    /// (e.g. `"macos"` → praxis `macos_remap()`). The named paradigm replaces a
    /// hand-listed remap table; defaults to `"macos"`. See [`Schema::remap_set`].
    #[serde(default)]
    pub paradigm: Option<String>,
}

/// A CapsLock-style dual-role layer (we read its tap/hold timing + target mode).
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Layer {
    #[serde(default)]
    pub hold: Option<String>,
    #[serde(default, rename = "tapHoldMs")]
    pub tap_hold_ms: Option<u64>,
    /// Idle auto-revert threshold (ms) for a STICKY (tapped/locked) mode entered
    /// via this layer — a forgotten lock self-heals. Falls back to
    /// [`DEFAULT_STICKY_IDLE_MS`].
    #[serde(default, rename = "stickyIdleMs")]
    pub sticky_idle_ms: Option<u64>,
    /// Engine-native: the mode this layer's trigger enters, named directly
    /// (e.g. `"desktop"`). This is how the vogix engine learns which mode
    /// CapsLock activates — no synthetic keysym indirection.
    #[serde(default, rename = "entersMode")]
    pub enters_mode: Option<String>,
    /// Legacy (kanata-era): the keysym the hold emitted (e.g. `"f23"`), bound in
    /// the root mode to a `submap, X` action whose `X` was the target. Superseded
    /// by `entersMode`; still read as a fallback for old schemas.
    #[serde(default, rename = "holdAction")]
    pub hold_action: Option<String>,
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

/// Default sticky-mode idle auto-revert threshold if the schema doesn't specify
/// one — a forgotten locked mode self-heals after this much inactivity.
pub const DEFAULT_STICKY_IDLE_MS: u64 = 30_000;

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

    /// Parse from a JSON string, then resolve the selected paradigm.
    pub fn from_json(text: &str) -> Result<Self> {
        let mut schema: Self = serde_json::from_str(text)
            .map_err(|e| VogixError::Config(format!("input schema parse error: {e}")))?;
        schema.resolve_paradigm();
        Ok(schema)
    }

    /// Resolve the selected paradigm's WM-nav modes into the schema.
    ///
    /// New-format schemas omit the mode graph + paradigm nav and carry only the
    /// `paradigm` selection name + the user's overlay modes; the engine projects
    /// the paradigm's `BindingSet` (see [`super::catalog`]) here and merges it
    /// UNDER the overlay (the user's own bindings win on a name collision). Legacy
    /// schemas ship a full mode graph and are left untouched.
    fn resolve_paradigm(&mut self) {
        if !self.mode_graph.modes.is_empty() {
            return;
        }
        if let Some((graph, nav_modes)) = super::catalog::resolve_paradigm(self.paradigm()) {
            for (mode, nav) in nav_modes {
                let entry = self.modes.entry(mode).or_default();
                for (name, binding) in nav.bindings {
                    entry.bindings.entry(name).or_insert(binding);
                }
            }
            self.mode_graph = graph;
        }
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

    /// The selected interaction paradigm (defaults to `"macos"`).
    pub fn paradigm(&self) -> &str {
        self.keybindings.paradigm.as_deref().unwrap_or("macos")
    }

    /// The Super-modifier remap set for the selected paradigm — a praxis
    /// [`RemapSet`] (cited + axiom-checkable), not a hand-listed table. `"macos"`
    /// → [`macos_remap`] (the full 21-letter Super→Ctrl set); `"copy-paste"` → a
    /// minimal Super+C/V → Ctrl+C/V set; `"none"` (or unknown) → an empty set.
    pub fn remap_set(&self) -> RemapSet {
        match super::catalog::paradigm_remap(self.paradigm()) {
            "macos" => macos_remap(),
            // A minimal terminal-aware copy/paste set: Super+C/V → Ctrl+C/V (and
            // Ctrl+Shift+C/V in terminals, via `terminal_copy_paste_target`).
            // Unlike the full macOS preset it touches ONLY C/V, so it never shadows
            // the flat WM Super-letter binds — and the engine keeps re-emitting
            // Super (super_passthrough) for pointer binds (Super+drag/scroll).
            "copy-paste" => {
                let mut rs = RemapSet::new("copy-paste");
                for c in ['c', 'v'] {
                    rs.add(
                        KeyCombo::new(Key::Letter(c)).with_mod(PxMod::Super),
                        KeyCombo::new(Key::Letter(c)).with_mod(PxMod::Ctrl),
                    );
                }
                rs
            }
            other => RemapSet::new(other),
        }
    }

    /// The sticky-mode idle auto-revert threshold (ms), from the dual-role layer.
    pub fn sticky_idle_ms(&self) -> u64 {
        self.keybindings
            .layers
            .values()
            .find(|l| l.hold.as_deref() == Some("capslock"))
            .and_then(|l| l.sticky_idle_ms)
            .unwrap_or(DEFAULT_STICKY_IDLE_MS)
    }

    /// The mode CapsLock enters.
    ///
    /// Engine-native: the `capslock` layer names the target mode directly via
    /// `entersMode`. For old (kanata-era) schemas that predate that field, fall
    /// back to the keysym indirection: the layer's `holdAction` keysym (e.g.
    /// `"f23"`) is bound in the root mode to a `submap, X` action, and `X` is the
    /// target. The fallback is unambiguous even when several modes are `submap`
    /// children of root (`move`/`resize` are too).
    pub fn caps_target(&self) -> Option<String> {
        let layer = self
            .keybindings
            .layers
            .values()
            .find(|l| l.hold.as_deref() == Some("capslock"))?;
        // Engine-native: the layer names its target mode.
        if let Some(target) = layer.enters_mode.as_ref() {
            return Some(target.clone());
        }
        // Legacy fallback: derive via the holdAction keysym → root-mode binding.
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

    /// Bindings (and remaps) whose key chord can't be parsed — a typo, or a key
    /// name the engine doesn't know. These would be SILENTLY DROPPED when the
    /// router is built (`parse_chord` → `None`), so a mistyped binding would just
    /// vanish with no error. Surfacing them lets `check` fail and `run` warn.
    /// Returns sorted human-readable `"<where>: <reason>"` descriptors.
    pub fn unparseable_bindings(&self) -> Vec<String> {
        let mut out = Vec::new();
        for (mode, spec) in &self.modes {
            for (name, b) in &spec.bindings {
                if super::keys::parse_chord(&b.key).is_none() {
                    out.push(format!("{mode}.{name}: unparseable key {:?}", b.key));
                }
            }
        }
        // Remaps are supplied by a praxis paradigm preset (always well-formed),
        // so there are no hand-written remap chords to validate here.
        out.sort();
        out
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
        // No paradigm set → defaults to macOS; the preset supplies the remaps.
        assert_eq!(s.paradigm(), "macos");
        assert!(s.remap_set().remaps.len() >= 6);
    }

    #[test]
    fn device_filter_merges_onto_safe_defaults() {
        // No deviceFilter in the schema → the baked-in baseline still excludes Yubico.
        assert!(schema().device_filter().exclude_vendors.contains(&0x1050));

        // A user deviceFilter EXTENDS (never replaces) the baseline — adding a
        // vendor cannot accidentally re-enable the YubiKey/audio grab.
        let custom = Schema::from_json(
            r#"{
              "modeGraph": { "root": "app", "modes": { "app": { "parent": null, "type": "normal" } } },
              "modes": { "app": { "bindings": {} } },
              "deviceFilter": { "excludeVendors": [2336], "excludeNameSubstrings": ["FooPad"] }
            }"#,
        )
        .unwrap();
        let f = custom.device_filter();
        assert!(f.exclude_vendors.contains(&0x1050), "baseline Yubico kept");
        assert!(f.exclude_vendors.contains(&2336), "user vendor added");
        assert!(f.exclude_name_substrings.iter().any(|s| s == "FooPad"));
        assert!(f.exclude_name_substrings.iter().any(|s| s == "Maonocaster"));

        // The Nix module renders a PRESENT-EMPTY object (`deviceFilter = ... or {}`),
        // which deserializes to Some(empty) — a DIFFERENT serde path than an absent
        // key (None). Both must keep the baked-in Yubico exclusion (merge, not replace).
        let empty = Schema::from_json(
            r#"{
              "modeGraph": { "root": "app", "modes": { "app": { "parent": null, "type": "normal" } } },
              "modes": { "app": { "bindings": {} } },
              "deviceFilter": {}
            }"#,
        )
        .unwrap();
        assert!(
            empty.device_filter().exclude_vendors.contains(&0x1050),
            "present-empty deviceFilter must still exclude Yubico (the rendered shape)"
        );
    }

    #[test]
    fn unparseable_bindings_are_surfaced_not_silently_dropped() {
        // The fixture's bindings all parse.
        assert!(schema().unparseable_bindings().is_empty());
        // A typo'd key name is surfaced — it would otherwise vanish silently when
        // the router is built (parse_chord → None → binding dropped).
        let bad = Schema::from_json(
            &FIXTURE.replace("\"key\": \"h\"", "\"key\": \"definitely-not-a-key\""),
        )
        .expect("still valid JSON");
        let probs = bad.unparseable_bindings();
        assert!(
            probs.iter().any(|s| s.contains("definitely-not-a-key")),
            "a mistyped key must be surfaced, got {probs:?}"
        );
    }

    #[test]
    fn caps_target_reads_enters_mode_engine_native() {
        // Engine-native schema: the caps layer names the mode directly via
        // `entersMode`, with NO synthetic `f23` keysym and NO `enterDesktopHold`
        // root binding. caps_target must resolve from the field alone.
        let json = r#"{
          "modeGraph": { "root": "app", "modes": {
            "app":     { "parent": null,  "type": "normal" },
            "desktop": { "parent": "app", "type": "submap" }
          }},
          "keybindings": { "modKey": "super", "layers": {
            "desktopToggle": { "hold": "capslock", "tapHoldMs": 250, "entersMode": "desktop" }
          }},
          "modes": {
            "app": { "exit": "escape", "bindings": {} },
            "desktop": { "exit": "escape", "bindings": {
              "focusLeft": { "key": "h", "action": "movefocus, l" }
            }}
          }
        }"#;
        let s = Schema::from_json(json).expect("engine-native fixture parses");
        assert_eq!(s.caps_target().as_deref(), Some("desktop"));
        // root -> desktop still derives from the parent topology (no f23 needed).
        assert!(s.build_mode_graph().validate().is_empty());
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

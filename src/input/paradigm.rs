//! Project a praxis keybinding `BindingSet` into vogix's loaded-schema parts.
//!
//! A praxis preset (`vim_preset()`, `emacs_preset()`, …) is a flat [`BindingSet`]:
//! a list of `(KeyCombo, ModeId, Action, repeat)` whose mode transitions are
//! encoded as `submap, X` *actions*, with NO explicit topology. vogix's
//! [`Schema`](super::schema::Schema) needs a mode GRAPH (root + per-mode
//! parent/kind). So a *paradigm* here = a `BindingSet` (praxis-sourced or
//! vogix-native) + a small vogix-authored [`Topology`] (the root + per-mode
//! parent/kind the preset can't express), projected into the `{ modeGraph, modes }`
//! the engine consumes.
//!
//! Kept self-contained so it can later move into praxis: once a praxis preset
//! carries its own topology, this becomes the praxis-runtime materialiser and the
//! `Topology` glue disappears.
use std::collections::HashMap;

use pr4xis_domains::applied::hmi::input::keybindings::{
    BindingSet, Key, KeyCombo, Modifier, MouseButton, NamedKey,
};

use super::schema::{Binding, ModeGraphSpec, ModeNode, ModeSpec};

/// One non-root mode's place in a paradigm's topology (the part a praxis
/// `BindingSet` cannot express — it only carries bindings + `submap` actions).
pub struct ModeTopo {
    pub name: &'static str,
    pub parent: &'static str,
    /// `"submap"` (catchall — swallows unbound keys) | `"normal"` | `"passthrough"`.
    pub kind: &'static str,
}

/// The per-paradigm mode topology vogix supplies alongside a `BindingSet`.
pub struct Topology {
    pub root: &'static str,
    /// Root mode kind (`"submap"` for modal paradigms whose root catches keys,
    /// `"passthrough"` for chorded desktop paradigms whose root passes keys on).
    pub root_kind: &'static str,
    pub modes: &'static [ModeTopo],
}

/// Render a praxis [`KeyCombo`] into vogix's key-chord string
/// (`"super + h"`, `"left"`, `"F12"`, `"shift + 0"`).
pub fn render_combo(combo: &KeyCombo) -> String {
    let mut parts: Vec<String> = combo
        .modifiers
        .iter()
        .map(|m| {
            match m {
                Modifier::Shift => "shift",
                Modifier::Ctrl => "ctrl",
                Modifier::Alt => "alt",
                Modifier::Super => "super",
                Modifier::Hyper => "hyper",
            }
            .to_string()
        })
        .collect();
    parts.push(match &combo.key {
        Key::Letter(c) => c.to_ascii_lowercase().to_string(),
        Key::Number(n) => n.to_string(),
        Key::Function(n) => format!("F{n}"),
        Key::Named(k) => render_named(k),
        Key::Mouse(b) => match b {
            // evdev button codes — vogix's `mouse:<code>` form (Hyprland `bindm`).
            MouseButton::Left => "mouse:272".to_string(),
            MouseButton::Right => "mouse:273".to_string(),
            MouseButton::Middle => "mouse:274".to_string(),
            MouseButton::ScrollUp => "mouse_up".to_string(),
            MouseButton::ScrollDown => "mouse_down".to_string(),
        },
    });
    parts.join(" + ")
}

/// Debug-name a [`NamedKey`] lowercased to vogix's convention (`Left` → `"left"`).
fn render_named(k: &NamedKey) -> String {
    format!("{k:?}").to_ascii_lowercase()
}

/// Project a `BindingSet` + [`Topology`] into the engine's `(modeGraph, modes)`.
///
/// The `modes` map is keyed by binding NAME (praxis `Action.name`, e.g.
/// `move_left`); each [`Binding`]'s `key` is the rendered chord. praxis carries
/// no one-shot flag — transitions ride `submap, X` actions — so `exitAfter` is
/// left `false` here; an overlay refines it.
pub fn project(bs: &BindingSet, topo: &Topology) -> (ModeGraphSpec, HashMap<String, ModeSpec>) {
    let mut graph_modes: HashMap<String, ModeNode> = HashMap::new();
    graph_modes.insert(
        topo.root.to_string(),
        ModeNode {
            parent: None,
            kind: Some(topo.root_kind.to_string()),
        },
    );
    for m in topo.modes {
        graph_modes.insert(
            m.name.to_string(),
            ModeNode {
                parent: Some(m.parent.to_string()),
                kind: Some(m.kind.to_string()),
            },
        );
    }
    let mode_graph = ModeGraphSpec {
        root: topo.root.to_string(),
        modes: graph_modes,
    };

    let mut modes: HashMap<String, ModeSpec> = HashMap::new();
    for b in &bs.bindings {
        let binding = Binding {
            key: render_combo(&b.combo),
            action: b.action.command.clone(),
            exit_after: false,
            repeat: b.repeat,
            description: Some(b.action.description.clone()),
        };
        modes
            .entry(b.mode.0.clone())
            .or_default()
            .bindings
            .insert(b.action.name.clone(), binding);
    }

    (mode_graph, modes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pr4xis_domains::applied::hmi::input::keybindings::vim_preset;

    // vogix authors vim's topology; praxis gives only the bindings.
    const VIM_TOPO: Topology = Topology {
        root: "normal",
        root_kind: "submap",
        modes: &[ModeTopo {
            name: "insert",
            parent: "normal",
            kind: "passthrough",
        }],
    };

    #[test]
    fn projects_vim_preset_faithfully() {
        let (graph, modes) = project(&vim_preset(), &VIM_TOPO);

        assert_eq!(graph.root, "normal");
        assert!(graph.modes.contains_key("insert"));
        assert_eq!(
            graph.modes["insert"].parent.as_deref(),
            Some("normal"),
            "insert's exit parent is normal"
        );

        let normal = modes.get("normal").expect("normal mode present");
        let h = normal.bindings.get("move_left").expect("move_left binding");
        assert_eq!(h.key, "h", "Key::Letter('h') renders as \"h\"");
        assert_eq!(h.action, "movefocus, l", "praxis command passes through");
        assert!(
            normal.bindings.contains_key("enter_insert"),
            "i → enter insert is projected"
        );

        let insert = modes.get("insert").expect("insert mode present");
        let esc = insert.bindings.get("exit_insert").expect("exit_insert");
        assert_eq!(esc.key, "escape", "NamedKey::Escape renders lowercase");
        assert_eq!(esc.action, "submap, reset");
    }

    #[test]
    fn renders_modifiers_and_named_keys() {
        let c = KeyCombo::new(Key::Named(NamedKey::Left)).with_mod(Modifier::Super);
        assert_eq!(render_combo(&c), "super + left");
        let n = KeyCombo::new(Key::Number(1)).with_mod(Modifier::Super);
        assert_eq!(render_combo(&n), "super + 1");
    }
}

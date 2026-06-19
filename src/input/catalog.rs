//! Resolve a paradigm SELECTION into vogix's loaded-schema parts.
//!
//! Every WM-navigation paradigm is now sourced from a praxis preset —
//! `vogix_preset()` / `windows_preset()` / `macos_preset()` / `linux_preset()`
//! (the lift), plus `vim` / `cua` / `emacs` / `i3`: praxis is the single source of
//! the bindings. This module pairs each praxis `BindingSet` with a small
//! vogix-authored [`Topology`] (the root + per-mode parent/kind a flat preset
//! can't express) and projects them into the `(modeGraph, modes)` the engine
//! consumes ([`super::paradigm::project`]). The `*_nav_preset()` fns are thin
//! aliases over the praxis presets, kept so the byte-identical guard still names
//! them.
//!
//! Scope: a paradigm `BindingSet` is the WM-NAVIGATION flavour only (focus / move
//! / resize / workspaces / window-state), over keys praxis's `Key` enum can
//! express. The user's own launch / system / media bindings — and any key praxis
//! can't represent (`super+slash`, `XF86Audio*`, `print`) — live in the vogix-side
//! OVERLAY, layered on top of whichever paradigm is selected.
use pr4xis_domains::applied::hmi::input::keybindings::{
    BindingSet, cua_preset, emacs_preset, i3_preset, linux_preset, macos_preset, vim_preset,
    vogix_preset, windows_preset,
};

use std::collections::HashMap;

use super::paradigm::{ModeTopo, Topology, project};
use super::schema::{ModeGraphSpec, ModeSpec};

/// Resolve a paradigm SELECTION name into its `(modeGraph, modes)` — the global
/// interaction model the engine materializes. Every entry is SYSTEM-WIDE: select
/// `cua` and `Ctrl+C` is globally copy, `i3` and `Super+hjkl` drives the WM. The
/// user's own launch/system/media keys are the OVERLAY, merged on top by the
/// engine ([`super::schema::Schema::resolve_paradigm`]).
///
/// Returns `None` for names that are NOT a known paradigm selection — notably the
/// legacy remap-name `paradigm` values (`"copy-paste"`/`"macos"`/`"none"`), whose
/// schemas already carry their modes inline and must not be re-resolved. The
/// engine only resolves when the schema omits its mode graph (the new format).
pub fn resolve_paradigm(name: &str) -> Option<(ModeGraphSpec, HashMap<String, ModeSpec>)> {
    match name {
        // All paradigms are praxis-sourced, projected through a vogix-authored
        // topology (the part a flat `BindingSet` can't express).
        "vogix" => Some(project(&vogix_nav_preset(), &VOGIX_TOPO)),
        "i3" => Some(project(&i3_preset(), &I3_TOPO)),
        "cua" => Some(project(&cua_preset(), &CUA_TOPO)),
        "emacs" => Some(project(&emacs_preset(), &EMACS_TOPO)),
        "vim" => Some(project(&vim_preset(), &VIM_TOPO)),
        "windows" => Some(project(&windows_nav_preset(), &WINDOWS_TOPO)),
        "macos" => Some(project(&macos_nav_preset(), &MACOS_TOPO)),
        "linux" => Some(project(&linux_nav_preset(), &LINUX_TOPO)),
        _ => None,
    }
}

/// The Super-modifier remap name for a paradigm SELECTION. Legacy remap-name
/// values pass through unchanged so [`super::schema::Schema::remap_set`] keeps
/// matching `"macos"`/`"copy-paste"`.
pub fn paradigm_remap(name: &str) -> &str {
    match name {
        "vogix" => "copy-paste",
        other => other,
    }
}

/// `vogix` is single-mode chorded; its topology is one passthrough `app` root.
pub const VOGIX_TOPO: Topology = Topology {
    root: "app",
    root_kind: "passthrough",
    modes: &[],
};

/// `cua` (the IBM/Windows shortcut standard) is single-mode chorded: Ctrl/Alt
/// shortcuts over a passthrough `app` root (unbound keys type normally).
pub const CUA_TOPO: Topology = Topology {
    root: "app",
    root_kind: "passthrough",
    modes: &[],
};

/// `emacs` is single-mode chorded: Ctrl/Meta prefixes over a passthrough `app`
/// root. (`C-x`-style multi-key prefixes aren't in the praxis preset.)
pub const EMACS_TOPO: Topology = Topology {
    root: "app",
    root_kind: "passthrough",
    modes: &[],
};

/// `i3` is a tiling-WM paradigm: a passthrough `app` root (Super-chorded
/// focus/move/workspace) plus a `resize` SUBMAP — `Super+r` enters it, `hjkl`
/// resize (catchall swallows other keys), `Escape` exits. The submode + its
/// enter/exit are the topology praxis's flat `BindingSet` cannot express.
pub const I3_TOPO: Topology = Topology {
    root: "app",
    root_kind: "passthrough",
    modes: &[ModeTopo {
        name: "resize",
        parent: "app",
        kind: "submap",
    }],
};

/// `vim` is MODAL — its root isn't `app`: `normal` is the catchall root (`hjkl`
/// nav; unbound keys are swallowed, never typed), and `i` enters the `insert`
/// SUBMODE (passthrough — keys type through), `Escape` returns. Because the root
/// is `normal`, the engine re-homes the user's overlay onto it (see
/// [`super::schema::Schema::resolve_paradigm`]) so the global keys stay reachable.
pub const VIM_TOPO: Topology = Topology {
    root: "normal",
    root_kind: "submap",
    modes: &[ModeTopo {
        name: "insert",
        parent: "normal",
        kind: "passthrough",
    }],
};

/// The desktop paradigms (`windows`/`macos`/`linux`) are single-mode chorded, like
/// `cua`: one passthrough `app` root (Super/Ctrl/Alt chords; unbound keys pass on).
pub const WINDOWS_TOPO: Topology = Topology {
    root: "app",
    root_kind: "passthrough",
    modes: &[],
};

/// See [`WINDOWS_TOPO`]. `macos` pairs this with the `macos` remap (Cmd-feel).
pub const MACOS_TOPO: Topology = Topology {
    root: "app",
    root_kind: "passthrough",
    modes: &[],
};

/// See [`WINDOWS_TOPO`]. `linux` = mainstream GNOME (floating, Super-based).
pub const LINUX_TOPO: Topology = Topology {
    root: "app",
    root_kind: "passthrough",
    modes: &[],
};

/// `vogix` — the house default WM-navigation layout, sourced from praxis's
/// `vogix_preset()` (the lift). This thin alias preserves the byte-identical guard
/// against the deployed layout.
pub fn vogix_nav_preset() -> BindingSet {
    vogix_preset()
}

/// `windows` — Microsoft Windows global window conventions, sourced from praxis's
/// `windows_preset()`. Win+Up maximize realizes to `fullscreen, 1` (the maximize
/// state), distinct from true fullscreen.
pub fn windows_nav_preset() -> BindingSet {
    windows_preset()
}

/// `macos` — Apple macOS Mission Control / Spaces / window conventions, sourced
/// from praxis's `macos_preset()` (pairs with the `macos` Cmd-feel remap; hide /
/// minimize are silent moves to special workspaces).
pub fn macos_nav_preset() -> BindingSet {
    macos_preset()
}

/// `linux` — mainstream GNOME Shell global window conventions, sourced from
/// praxis's `linux_preset()`. Super+Up maximize realizes to `fullscreen, 1`.
pub fn linux_nav_preset() -> BindingSet {
    linux_preset()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::paradigm::project;

    #[test]
    fn vogix_nav_projects_with_expected_focus_and_workspace_binds() {
        let (graph, modes) = project(&vogix_nav_preset(), &VOGIX_TOPO);
        assert_eq!(graph.root, "app");
        let app = modes.get("app").expect("app mode");

        // Focus left via both Super+h and Super+Left → movefocus, l.
        assert_eq!(app.bindings["focus_l"].key, "super + h");
        assert_eq!(app.bindings["focus_l"].action, "movefocus, l");
        assert_eq!(app.bindings["focus_l_arrow"].key, "super + left");
        assert_eq!(app.bindings["focus_l_arrow"].action, "movefocus, l");

        // Resize repeats; Super+0 = workspace 10.
        assert!(app.bindings["resize_l"].repeat);
        assert_eq!(app.bindings["workspace_10"].key, "super + 0");
        assert_eq!(app.bindings["workspace_10"].action, "workspace, 10");

        // Window-state row present.
        assert_eq!(app.bindings["close"].action, "killactive,");
        assert_eq!(app.bindings["group_cycle"].key, "super + tab");
    }

    #[test]
    fn engine_resolves_new_format_vogix_under_overlay() {
        use crate::input::schema::Schema;
        // New format: selection name + overlay only, NO modeGraph → engine resolves.
        let json = r#"{
            "keybindings": { "paradigm": "vogix" },
            "modes": { "app": { "bindings": {
                "launcher": { "key": "super + space", "action": "exec, walker" }
            } } }
        }"#;
        let schema = Schema::from_json(json).expect("parse + resolve");
        assert_eq!(schema.mode_graph.root, "app", "nav mode graph resolved");
        let app = &schema.modes["app"];
        assert_eq!(
            app.bindings["launcher"].action, "exec, walker",
            "overlay kept"
        );
        assert_eq!(app.bindings["focus_l"].key, "super + h", "nav merged in");
        assert_eq!(app.bindings["workspace_10"].action, "workspace, 10");
    }

    /// The byte-identical guard: every projected `vogix` nav binding must have an
    /// identical action in the deployed live layout, matched by PARSED chord (so
    /// modifier order in the string is irrelevant). Catches any mistranslation
    /// before the engine-resolved `vogix` can replace the Nix-resolved one.
    #[test]
    fn vogix_nav_is_byte_identical_to_live_layout() {
        use crate::input::keys::parse_chord;
        use crate::input::schema::Schema;

        // Deployed schema (old format → from_json does NOT re-resolve; loaded as-is).
        let live = Schema::from_json(include_str!("../../tests/fixtures/vogix-input.json"))
            .expect("fixture parses");
        let live_app = &live.modes["app"].bindings;

        let (_, nav) = project(&vogix_nav_preset(), &VOGIX_TOPO);
        for b in nav["app"].bindings.values() {
            let chord =
                parse_chord(&b.key).unwrap_or_else(|| panic!("nav chord {:?} parses", b.key));
            let lb = live_app
                .values()
                .find(|lb| parse_chord(&lb.key).as_ref() == Some(&chord))
                .unwrap_or_else(|| panic!("live layout has no binding for nav {:?}", b.key));
            assert_eq!(
                lb.action, b.action,
                "nav {:?}: live action {:?} != preset action {:?}",
                b.key, lb.action, b.action
            );
        }
    }

    /// The full flip proof: the engine-resolved `vogix` (the new-format overlay +
    /// the resolved nav) equals the deployed live layout by chord→action. Guards
    /// the defaults.nix split — no nav left in the overlay, no overlay binding lost.
    #[test]
    fn engine_resolved_vogix_equals_the_live_layout() {
        use crate::input::schema::Schema;
        use std::collections::BTreeMap;

        // Chord (modifier-order-independent) → action, for the app mode.
        fn norm_map(s: &Schema) -> BTreeMap<String, String> {
            s.modes["app"]
                .bindings
                .values()
                .map(|b| {
                    let mut parts: Vec<&str> = b.key.split(" + ").collect();
                    parts.sort_unstable();
                    (parts.join(" + "), b.action.clone())
                })
                .collect()
        }

        let mut resolved = norm_map(
            &Schema::from_json(include_str!(
                "../../tests/fixtures/vogix-input-overlay.json"
            ))
            .expect("new-format overlay resolves"),
        );
        let mut live = norm_map(
            &Schema::from_json(include_str!("../../tests/fixtures/vogix-input.json"))
                .expect("live fixture"),
        );

        // The ONE intended change of the flip: the help binding now invokes the
        // engine view (`vogix input keys`) instead of the build-time Nix script.
        let help = "slash + super".to_string();
        assert_eq!(
            resolved.remove(&help).as_deref(),
            Some("exec, vogix input keys")
        );
        assert_eq!(
            live.remove(&help).as_deref(),
            Some("exec, vogix-modes-global")
        );

        // Every other binding — all nav + overlay — is byte-identical.
        assert_eq!(
            resolved, live,
            "engine-resolved vogix must equal the live layout (modulo the help binding)"
        );
    }

    // ── praxis-sourced paradigms (global, projected via a vogix topology) ──

    #[test]
    fn cua_resolves_as_a_global_ctrl_shortcut_layer() {
        let (graph, modes) = project(&cua_preset(), &CUA_TOPO);
        assert_eq!(graph.root, "app");
        let app = modes.get("app").expect("app mode");
        assert_eq!(app.bindings["copy"].key, "ctrl + c");
        assert_eq!(app.bindings["copy"].action, "exec, wl-copy");
        assert_eq!(app.bindings["quit"].key, "alt + F4");
        assert_eq!(app.bindings["switch_window"].action, "cyclenext,");
    }

    #[test]
    fn emacs_resolves_with_ctrl_meta_movement() {
        let (graph, modes) = project(&emacs_preset(), &EMACS_TOPO);
        assert_eq!(graph.root, "app");
        let app = modes.get("app").expect("app mode");
        assert_eq!(app.bindings["forward_char"].key, "ctrl + f");
        assert_eq!(app.bindings["forward_char"].action, "movefocus, r");
        assert_eq!(app.bindings["forward_word"].key, "alt + f");
        assert_eq!(app.bindings["cancel"].action, "submap, reset");
    }

    #[test]
    fn i3_resolves_with_a_resize_submap() {
        let (graph, modes) = project(&i3_preset(), &I3_TOPO);
        assert_eq!(graph.root, "app");
        assert_eq!(graph.modes["resize"].parent.as_deref(), Some("app"));
        assert_eq!(graph.modes["resize"].kind.as_deref(), Some("submap"));

        let app = modes.get("app").expect("app mode");
        assert_eq!(app.bindings["focus_left"].key, "super + h");
        assert_eq!(app.bindings["focus_left"].action, "movefocus, l");
        assert_eq!(app.bindings["enter_resize"].action, "submap, resize");

        let resize = modes.get("resize").expect("resize mode");
        assert_eq!(resize.bindings["resize_h"].action, "resizeactive, -30 0");
        assert!(resize.bindings["resize_h"].repeat);
        assert_eq!(resize.bindings["exit_resize"].action, "submap, reset");
    }

    #[test]
    fn i3_mode_graph_validates_under_the_praxis_axioms() {
        use crate::input::schema::Schema;
        // The engine derives the praxis ModeGraph from the resolved topology; the
        // app + resize-submap graph must satisfy the same axioms the vogix one does.
        let json = r#"{
            "keybindings": { "paradigm": "i3" },
            "modes": { "app": { "bindings": {} } }
        }"#;
        let schema = Schema::from_json(json).expect("i3 resolves");
        assert!(
            schema.build_mode_graph().validate().is_empty(),
            "i3 (app + resize submap) must satisfy the praxis mode-graph axioms"
        );
    }

    #[test]
    fn overlay_wins_chord_collision_when_selecting_i3() {
        use crate::input::keys::parse_chord;
        use crate::input::schema::Schema;
        // i3 binds Super+D = launcher; the overlay binds Super+D = dismiss (a
        // DIFFERENT name — so only the chord collides, isolating the chord policy).
        // Global paradigm, but the user's own key wins: Super+D stays dismiss and
        // i3's colliding launcher is dropped; i3's non-colliding nav stays.
        let json = r#"{
            "keybindings": { "paradigm": "i3" },
            "modes": { "app": { "bindings": {
                "dismiss": { "key": "super + d", "action": "exec, makoctl dismiss" }
            } } }
        }"#;
        let schema = Schema::from_json(json).expect("i3 + overlay resolves");
        let app = &schema.modes["app"].bindings;

        let super_d = parse_chord("super + d").unwrap();
        let on_super_d: Vec<&str> = app
            .values()
            .filter(|b| parse_chord(&b.key) == Some(super_d))
            .map(|b| b.action.as_str())
            .collect();
        assert_eq!(
            on_super_d,
            vec!["exec, makoctl dismiss"],
            "the overlay owns Super+D; i3's launcher-on-Super+D was dropped"
        );
        // i3 nav that doesn't collide with the overlay survives.
        assert_eq!(app["focus_left"].action, "movefocus, l");
    }

    // ── modal paradigm: vim (root is `normal`, not `app`) ──

    #[test]
    fn vim_resolves_as_modal_normal_insert() {
        let (graph, modes) = project(&vim_preset(), &VIM_TOPO);
        assert_eq!(
            graph.root, "normal",
            "vim's root is the catchall normal mode"
        );
        assert_eq!(graph.modes["insert"].parent.as_deref(), Some("normal"));
        assert_eq!(graph.modes["insert"].kind.as_deref(), Some("passthrough"));

        let normal = modes.get("normal").expect("normal mode");
        assert_eq!(normal.bindings["move_left"].key, "h");
        assert_eq!(normal.bindings["move_left"].action, "movefocus, l");
        assert_eq!(normal.bindings["enter_insert"].action, "submap, insert");

        let insert = modes.get("insert").expect("insert mode");
        assert_eq!(insert.bindings["exit_insert"].action, "submap, reset");
    }

    #[test]
    fn overlay_rehomes_to_paradigm_root_for_vim() {
        use crate::input::schema::Schema;
        // vim's root is `normal`, not `app`. The user's overlay (authored under
        // `app`) must re-home onto `normal` so the global keys stay reachable —
        // otherwise they'd sit in an `app` mode the vim graph never enters.
        let json = r#"{
            "keybindings": { "paradigm": "vim" },
            "modes": { "app": { "bindings": {
                "launcher": { "key": "super + space", "action": "exec, walker" }
            } } }
        }"#;
        let schema = Schema::from_json(json).expect("vim + overlay resolves");
        assert_eq!(schema.mode_graph.root, "normal");
        assert!(
            !schema.modes.contains_key("app"),
            "the orphan `app` overlay mode is gone — it re-homed onto `normal`"
        );
        let normal = &schema.modes["normal"].bindings;
        assert_eq!(
            normal["launcher"].action, "exec, walker",
            "overlay re-homed onto the vim root"
        );
        assert_eq!(
            normal["move_left"].action, "movefocus, l",
            "vim nav present"
        );
    }

    #[test]
    fn vim_mode_graph_validates_under_the_praxis_axioms() {
        use crate::input::schema::Schema;
        let json = r#"{
            "keybindings": { "paradigm": "vim" },
            "modes": { "app": { "bindings": {} } }
        }"#;
        let schema = Schema::from_json(json).expect("vim resolves");
        assert!(
            schema.build_mode_graph().validate().is_empty(),
            "vim (normal root + insert submode) must satisfy the praxis axioms"
        );
    }

    // ── vogix-authored desktop paradigms (windows / macos / linux) ──

    #[test]
    fn windows_resolves_with_snap_and_virtual_desktops() {
        let (graph, modes) = project(&windows_nav_preset(), &WINDOWS_TOPO);
        assert_eq!(graph.root, "app");
        let app = modes.get("app").expect("app mode");
        assert_eq!(app.bindings["switch_window"].action, "cyclenext,");
        assert_eq!(app.bindings["close"].action, "killactive,");
        assert_eq!(app.bindings["snap_left"].action, "movewindow, l");
        assert_eq!(app.bindings["maximize"].action, "fullscreen, 1");
        assert_eq!(app.bindings["desktop_next"].action, "workspace, +1");
        assert_eq!(app.bindings["workspace_1"].action, "workspace, 1");
        // WM-nav only — no app-launch leaked into the paradigm.
        assert!(!app.bindings.contains_key("launcher"));
    }

    #[test]
    fn macos_resolves_and_selects_the_cmd_feel_remap() {
        use crate::input::schema::Schema;
        let (graph, modes) = project(&macos_nav_preset(), &MACOS_TOPO);
        assert_eq!(graph.root, "app");
        let app = modes.get("app").expect("app mode");
        assert_eq!(app.bindings["workspace_prev"].action, "workspace, -1");
        assert_eq!(
            app.bindings["mission_control"].action,
            "togglespecialworkspace, overview"
        );
        assert_eq!(app.bindings["close_window"].action, "killactive,");
        assert_eq!(app.bindings["fullscreen"].action, "fullscreen");

        // The macos paradigm selects the praxis macos_remap (the Cmd-feel). The
        // bound window verbs (Cmd+W/Q/H/M) take precedence over it; the rest of
        // Super+letter remaps to Ctrl.
        let schema = Schema::from_json(
            r#"{ "keybindings": { "paradigm": "macos" }, "modes": { "app": { "bindings": {} } } }"#,
        )
        .expect("macos resolves");
        assert!(
            schema.remap_set().remaps.len() >= 6,
            "macos paradigm uses the full Cmd-feel remap"
        );
    }

    #[test]
    fn linux_resolves_with_pageup_pagedown_workspace_nav() {
        use crate::input::keys::parse_chord;
        let (graph, modes) = project(&linux_nav_preset(), &LINUX_TOPO);
        assert_eq!(graph.root, "app");
        let app = modes.get("app").expect("app mode");
        // The GNOME PageUp/PageDown workspace verbs — the chord must PARSE (the gap
        // in keys.rs that would otherwise silently drop them).
        assert_eq!(app.bindings["workspace_prev"].key, "super + pageup");
        assert!(parse_chord("super + pageup").is_some(), "pageup must parse");
        assert_eq!(app.bindings["workspace_next"].action, "workspace, +1");
        assert_eq!(app.bindings["move_to_prev"].action, "movetoworkspace, -1");
        assert_eq!(app.bindings["tile_left"].action, "movewindow, l");
        // GNOME's Super+1..9 is app-LAUNCH, deliberately omitted (overlay scope).
        assert!(!app.bindings.contains_key("workspace_1"));
    }

    #[test]
    fn desktop_paradigms_parse_and_validate() {
        use crate::input::schema::Schema;
        // Every desktop paradigm's chords must PARSE (else they'd be silently
        // dropped when the router is built), and its derived mode graph must
        // satisfy the praxis axioms.
        for name in ["windows", "macos", "linux"] {
            let json = format!(
                r#"{{ "keybindings": {{ "paradigm": "{name}" }}, "modes": {{ "app": {{ "bindings": {{}} }} }} }}"#
            );
            let schema =
                Schema::from_json(&json).unwrap_or_else(|e| panic!("{name} resolves: {e}"));
            assert!(
                schema.unparseable_bindings().is_empty(),
                "{name} has unparseable chords: {:?}",
                schema.unparseable_bindings()
            );
            assert!(
                schema.build_mode_graph().validate().is_empty(),
                "{name} mode graph must satisfy the praxis axioms"
            );
        }
    }
}

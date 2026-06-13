//! vogix-native interaction paradigms as praxis-shaped [`BindingSet`]s.
//!
//! These are the paradigms praxis does not ship a preset for: the desktop-chorded
//! `windows` / `macos` / `linux`, and `vogix` (the house default = your live
//! layout's WM-nav). They are authored here with the SAME praxis types as the
//! consumed presets (`vim_preset()` etc.) so every paradigm projects uniformly
//! ([`super::paradigm::project`]) and these can later move into praxis.
//!
//! Scope: a paradigm `BindingSet` is the WM-NAVIGATION flavour only (focus / move
//! / resize / workspaces / window-state), over keys praxis's `Key` enum can
//! express. The user's own launch / system / media bindings — and any key praxis
//! can't represent (`super+slash`, `XF86Audio*`, `print`) — live in the vogix-side
//! OVERLAY, layered on top of whichever paradigm is selected.
use pr4xis_domains::applied::hmi::input::keybindings::{
    Action, BindingSet, Key, KeyCombo, Modifier,
};
use pr4xis_domains::applied::hmi::input::modes::ModeId;

use std::collections::HashMap;

use super::paradigm::{Topology, project};
use super::schema::{ModeGraphSpec, ModeSpec};

use Modifier::{Ctrl, Shift, Super};

/// Resolve a paradigm SELECTION name into its WM-nav `(modeGraph, modes)`.
///
/// Returns `None` for names that are NOT a known paradigm selection — notably the
/// legacy remap-name `paradigm` values (`"copy-paste"`/`"macos"`/`"none"`), whose
/// schemas already carry their modes inline and must not be re-resolved. The
/// engine only resolves when the schema omits its mode graph (the new format).
pub fn resolve_paradigm(name: &str) -> Option<(ModeGraphSpec, HashMap<String, ModeSpec>)> {
    match name {
        "vogix" => Some(project(&vogix_nav_preset(), &VOGIX_TOPO)),
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

/// Build a [`KeyCombo`] from modifiers + a key.
fn combo(mods: &[Modifier], key: Key) -> KeyCombo {
    let mut c = KeyCombo::new(key);
    for m in mods {
        c = c.with_mod(*m);
    }
    c
}

/// The four cardinal directions as `(letter, arrow-name, hypr-suffix)`. The live
/// layout binds BOTH `hjkl` and the arrows to the same action (`h`=left, `l`=right,
/// `j`=up, `k`=down — the user's own non-vim mapping), so every nav verb is
/// generated for both. Arrow names use praxis `NamedKey` (Left/Right/Up/Down).
const DIRS: &[(char, NamedDir, &str)] = &[
    ('h', NamedDir::Left, "l"),
    ('l', NamedDir::Right, "r"),
    ('j', NamedDir::Up, "u"),
    ('k', NamedDir::Down, "d"),
];

#[derive(Clone, Copy)]
enum NamedDir {
    Left,
    Right,
    Up,
    Down,
}

impl NamedDir {
    fn key(self) -> Key {
        use pr4xis_domains::applied::hmi::input::keybindings::NamedKey;
        Key::Named(match self {
            NamedDir::Left => NamedKey::Left,
            NamedDir::Right => NamedKey::Right,
            NamedDir::Up => NamedKey::Up,
            NamedDir::Down => NamedKey::Down,
        })
    }
}

/// `vogix` — the house default: the user's live WM-navigation layout, flat
/// `Super`-combos in one `app` mode. Byte-equivalent (by parsed chord+action) to
/// the nav half of `nix/modules/behavior/defaults.nix`; the launch/system/media
/// half is the overlay. Guarded by an equivalence test against the Nix output.
pub fn vogix_nav_preset() -> BindingSet {
    let mut bs = BindingSet::new("vogix");
    let app = ModeId::new("app");
    let mut add = |mods: &[Modifier], key: Key, name: &str, desc: &str, cmd: &str, repeat: bool| {
        bs.add(
            combo(mods, key),
            app.clone(),
            Action::new(name, desc, cmd),
            repeat,
        );
    };

    // ── Focus (Super + dir → movefocus); hjkl AND arrows ──
    for (letter, arrow, suf) in DIRS {
        let cmd = format!("movefocus, {suf}");
        add(
            &[Super],
            Key::Letter(*letter),
            &format!("focus_{suf}"),
            "Focus",
            &cmd,
            false,
        );
        add(
            &[Super],
            arrow.key(),
            &format!("focus_{suf}_arrow"),
            "Focus",
            &cmd,
            false,
        );
    }
    // ── Move window (Super + Shift + dir → swapwindow) ──
    for (letter, arrow, suf) in DIRS {
        let cmd = format!("swapwindow, {suf}");
        add(
            &[Super, Shift],
            Key::Letter(*letter),
            &format!("move_{suf}"),
            "Move window",
            &cmd,
            false,
        );
        add(
            &[Super, Shift],
            arrow.key(),
            &format!("move_{suf}_arrow"),
            "Move window",
            &cmd,
            false,
        );
    }
    // ── Resize window (Ctrl + Shift + dir → resizeactive; repeats) ──
    let resize_deltas = [("l", "-30 0"), ("r", "30 0"), ("u", "0 -30"), ("d", "0 30")];
    for (i, (letter, arrow, suf)) in DIRS.iter().enumerate() {
        let cmd = format!("resizeactive, {}", resize_deltas[i].1);
        add(
            &[Ctrl, Shift],
            Key::Letter(*letter),
            &format!("resize_{suf}"),
            "Resize",
            &cmd,
            true,
        );
        add(
            &[Ctrl, Shift],
            arrow.key(),
            &format!("resize_{suf}_arrow"),
            "Resize",
            &cmd,
            true,
        );
    }

    // ── Window state (the yuiop row + q/tab) ──
    add(
        &[Super],
        Key::Letter('q'),
        "close",
        "Close window",
        "killactive,",
        false,
    );
    add(
        &[Super],
        Key::Letter('y'),
        "float_pin",
        "Float + pin",
        "exec, hyprctl dispatch togglefloating ; hyprctl dispatch pin",
        false,
    );
    add(
        &[Super],
        Key::Letter('f'),
        "fullscreen",
        "Fullscreen",
        "fullscreen",
        false,
    );
    add(
        &[Super],
        Key::Letter('p'),
        "pseudo",
        "Pseudotile",
        "pseudo,",
        false,
    );
    add(
        &[Super],
        Key::Letter('o'),
        "toggle_split",
        "Toggle split",
        "layoutmsg, togglesplit",
        false,
    );
    add(
        &[Super],
        Key::Letter('u'),
        "toggle_group",
        "Toggle group",
        "togglegroup,",
        false,
    );
    {
        use pr4xis_domains::applied::hmi::input::keybindings::NamedKey;
        add(
            &[Super],
            Key::Named(NamedKey::Tab),
            "group_cycle",
            "Cycle window in group",
            "changegroupactive, f",
            false,
        );
    }

    // ── Workspaces (Super + number; 0 = ws 10) ──
    for n in 1u8..=10 {
        let key = Key::Number(if n == 10 { 0 } else { n });
        add(
            &[Super],
            key,
            &format!("workspace_{n}"),
            "Workspace",
            &format!("workspace, {n}"),
            false,
        );
    }
    add(
        &[Super],
        Key::Letter('m'),
        "workspace_music",
        "Music workspace",
        "workspace, Music",
        false,
    );

    // ── Adjacent workspace (Super + Ctrl + ←/→ or j/l) ──
    {
        use pr4xis_domains::applied::hmi::input::keybindings::NamedKey;
        add(
            &[Super, Ctrl],
            Key::Named(NamedKey::Left),
            "ws_prev",
            "Previous workspace",
            "workspace, -1",
            false,
        );
        add(
            &[Super, Ctrl],
            Key::Named(NamedKey::Right),
            "ws_next",
            "Next workspace",
            "workspace, +1",
            false,
        );
    }
    add(
        &[Super, Ctrl],
        Key::Letter('j'),
        "ws_prev_j",
        "Previous workspace",
        "workspace, -1",
        false,
    );
    add(
        &[Super, Ctrl],
        Key::Letter('l'),
        "ws_next_l",
        "Next workspace",
        "workspace, +1",
        false,
    );

    // ── Send window to workspace (Super + Ctrl + number) ──
    for n in 1u8..=10 {
        let key = Key::Number(if n == 10 { 0 } else { n });
        add(
            &[Super, Ctrl],
            key,
            &format!("move_to_ws_{n}"),
            "Send window to workspace",
            &format!("movetoworkspace, {n}"),
            false,
        );
    }
    // ── Send window to adjacent workspace (Super + Ctrl + Shift + ←/→ or j/l) ──
    {
        use pr4xis_domains::applied::hmi::input::keybindings::NamedKey;
        add(
            &[Super, Ctrl, Shift],
            Key::Named(NamedKey::Left),
            "send_ws_prev",
            "Send window ← workspace",
            "movetoworkspace, -1",
            false,
        );
        add(
            &[Super, Ctrl, Shift],
            Key::Named(NamedKey::Right),
            "send_ws_next",
            "Send window → workspace",
            "movetoworkspace, +1",
            false,
        );
    }
    add(
        &[Super, Ctrl, Shift],
        Key::Letter('j'),
        "send_ws_prev_j",
        "Send window ← workspace",
        "movetoworkspace, -1",
        false,
    );
    add(
        &[Super, Ctrl, Shift],
        Key::Letter('l'),
        "send_ws_next_l",
        "Send window → workspace",
        "movetoworkspace, +1",
        false,
    );

    // ── Send window to workspace silently (Super + Shift + number) ──
    for n in 1u8..=10 {
        let key = Key::Number(if n == 10 { 0 } else { n });
        add(
            &[Super, Shift],
            key,
            &format!("move_silent_{n}"),
            "Send window to workspace (silent)",
            &format!("movetoworkspacesilent, {n}"),
            false,
        );
    }

    bs
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
}

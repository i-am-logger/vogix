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
    Action, BindingSet, Key, KeyCombo, Modifier, cua_preset, emacs_preset, i3_preset, vim_preset,
};
use pr4xis_domains::applied::hmi::input::modes::ModeId;

use std::collections::HashMap;

use super::paradigm::{ModeTopo, Topology, project};
use super::schema::{ModeGraphSpec, ModeSpec};

use Modifier::{Alt, Ctrl, Shift, Super};

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
        // vogix-authored: the house default (the user's live WM-nav layout).
        "vogix" => Some(project(&vogix_nav_preset(), &VOGIX_TOPO)),
        // praxis-sourced presets, projected through a vogix-authored topology.
        "i3" => Some(project(&i3_preset(), &I3_TOPO)),
        "cua" => Some(project(&cua_preset(), &CUA_TOPO)),
        "emacs" => Some(project(&emacs_preset(), &EMACS_TOPO)),
        "vim" => Some(project(&vim_preset(), &VIM_TOPO)),
        // vogix-authored desktop paradigms (praxis has no preset yet — these are
        // written in the praxis mold so they can move into praxis later).
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

/// `windows` — Microsoft Windows 11 global window & virtual-desktop keyboard
/// conventions, projected to Hyprland. Single passthrough `app` mode, NO remap
/// (Windows uses Ctrl natively for the clipboard). Hyprland has no snap / maximize
/// / minimize, so those conventions are ADAPTED to the nearest real dispatcher
/// (snap → directional `movewindow`, maximize → `fullscreen`); Win+D / Win+M /
/// Win+Tab are omitted (no Hyprland show-desktop / minimize / overview). Win+1..9
/// (natively a taskbar-app launcher) is treated as virtual-desktop N.
///
/// Source: Microsoft Support, "Keyboard shortcuts in Windows".
pub fn windows_nav_preset() -> BindingSet {
    use pr4xis_domains::applied::hmi::input::keybindings::NamedKey;
    let mut bs = BindingSet::new("windows");
    let app = ModeId::new("app");
    let mut add = |mods: &[Modifier], key: Key, name: &str, desc: &str, cmd: &str| {
        bs.add(
            combo(mods, key),
            app.clone(),
            Action::new(name, desc, cmd),
            false,
        );
    };

    // Window switch (Alt+Tab) + close (Alt+F4) — faithful.
    add(
        &[Alt],
        Key::Named(NamedKey::Tab),
        "switch_window",
        "Switch window",
        "cyclenext,",
    );
    add(
        &[Alt, Shift],
        Key::Named(NamedKey::Tab),
        "switch_window_prev",
        "Switch window (reverse)",
        "cyclenext, prev",
    );
    add(
        &[Alt],
        Key::Function(4),
        "close",
        "Close window",
        "killactive,",
    );

    // Snap (Win+←/→) + maximize (Win+↑) — adapted (no snap/maximize in Hyprland).
    add(
        &[Super],
        Key::Named(NamedKey::Left),
        "snap_left",
        "Snap window left",
        "movewindow, l",
    );
    add(
        &[Super],
        Key::Named(NamedKey::Right),
        "snap_right",
        "Snap window right",
        "movewindow, r",
    );
    add(
        &[Super],
        Key::Named(NamedKey::Up),
        "maximize",
        "Maximize window",
        "fullscreen",
    );

    // Move window (Win+Shift+arrows: monitor/stretch/restore) — adapted to a
    // directional move (Hyprland has no monitor-move / vertical-stretch / restore).
    add(
        &[Super, Shift],
        Key::Named(NamedKey::Left),
        "move_left",
        "Move window left",
        "movewindow, l",
    );
    add(
        &[Super, Shift],
        Key::Named(NamedKey::Right),
        "move_right",
        "Move window right",
        "movewindow, r",
    );
    add(
        &[Super, Shift],
        Key::Named(NamedKey::Up),
        "move_up",
        "Move window up",
        "movewindow, u",
    );
    add(
        &[Super, Shift],
        Key::Named(NamedKey::Down),
        "move_down",
        "Move window down",
        "movewindow, d",
    );

    // Virtual desktops: Ctrl+Win+←/→ switch; Win+1..9 → desktop N.
    add(
        &[Super, Ctrl],
        Key::Named(NamedKey::Left),
        "desktop_prev",
        "Previous virtual desktop",
        "workspace, -1",
    );
    add(
        &[Super, Ctrl],
        Key::Named(NamedKey::Right),
        "desktop_next",
        "Next virtual desktop",
        "workspace, +1",
    );
    for n in 1u8..=9 {
        add(
            &[Super],
            Key::Number(n),
            &format!("workspace_{n}"),
            "Virtual desktop",
            &format!("workspace, {n}"),
        );
    }
    bs
}

/// `macos` — Apple macOS global Mission Control / Spaces / window conventions,
/// projected to Hyprland. Pairs with the `macos` remap (the Cmd-feel: Super+letter
/// → Ctrl+letter) for app shortcuts; the window verbs here (Cmd+W/Q/H/M) are
/// BOUND, so they take precedence over the remap for those letters (bindings win —
/// see `device.rs`), while the rest of Cmd+letter remaps to Ctrl. Spaces / Mission
/// Control are natively Ctrl-based (not remapped), and Cmd+Tab uses NamedKey Tab
/// (also untouched). Hyprland has no Mission Control / hide / minimize, so those
/// are ADAPTED (overview / hidden+minimized special workspaces).
///
/// Source: Apple Support, "Mac keyboard shortcuts" & "Use Mission Control".
pub fn macos_nav_preset() -> BindingSet {
    use pr4xis_domains::applied::hmi::input::keybindings::NamedKey;
    let mut bs = BindingSet::new("macos");
    let app = ModeId::new("app");
    let mut add = |mods: &[Modifier], key: Key, name: &str, desc: &str, cmd: &str| {
        bs.add(
            combo(mods, key),
            app.clone(),
            Action::new(name, desc, cmd),
            false,
        );
    };

    // Spaces: Ctrl+←/→ + Ctrl+1..9 (native Ctrl — untouched by the Cmd remap).
    add(
        &[Ctrl],
        Key::Named(NamedKey::Left),
        "workspace_prev",
        "Previous Space",
        "workspace, -1",
    );
    add(
        &[Ctrl],
        Key::Named(NamedKey::Right),
        "workspace_next",
        "Next Space",
        "workspace, +1",
    );
    for n in 1u8..=9 {
        add(
            &[Ctrl],
            Key::Number(n),
            &format!("workspace_{n}"),
            "Space",
            &format!("workspace, {n}"),
        );
    }
    // Mission Control (Ctrl+↑) — adapted to an `overview` special workspace.
    add(
        &[Ctrl],
        Key::Named(NamedKey::Up),
        "mission_control",
        "Mission Control",
        "togglespecialworkspace, overview",
    );

    // Window switch (Cmd+Tab) — Tab is not a letter, so not remapped.
    add(
        &[Super],
        Key::Named(NamedKey::Tab),
        "switch_window",
        "Switch window",
        "cyclenext,",
    );
    add(
        &[Super, Shift],
        Key::Named(NamedKey::Tab),
        "switch_window_prev",
        "Switch window (reverse)",
        "cyclenext, prev",
    );

    // Window verbs (Cmd+W/Q/H/M) — bound, so they win over the remap for w/q/h/m.
    add(
        &[Super],
        Key::Letter('w'),
        "close_window",
        "Close window",
        "killactive,",
    );
    add(
        &[Super],
        Key::Letter('q'),
        "quit",
        "Quit app",
        "killactive,",
    );
    add(
        &[Super],
        Key::Letter('h'),
        "hide",
        "Hide window",
        "movetoworkspacesilent, special:hidden",
    );
    add(
        &[Super],
        Key::Letter('m'),
        "minimize",
        "Minimize window",
        "movetoworkspacesilent, special:minimized",
    );
    // Fullscreen (Ctrl+Cmd+F) — two modifiers, so not remapped.
    add(
        &[Ctrl, Super],
        Key::Letter('f'),
        "fullscreen",
        "Toggle fullscreen",
        "fullscreen",
    );
    bs
}

/// `linux` — mainstream GNOME Shell global window conventions, projected to
/// Hyprland. Single passthrough `app` mode, NO remap (native Ctrl). The faithful
/// workspace verbs are the relative Super+PageUp/PageDown pair; GNOME's per-index
/// Super+1..9 is app-LAUNCH (`switch-to-application-N`), NOT workspace switching,
/// so it is deliberately omitted (it would belong in the user's overlay). Hyprland
/// has no snap / maximize / minimize, so tile / maximize / hide are ADAPTED.
///
/// Source: GNOME Shell defaults (`org.gnome.desktop.wm.keybindings`).
pub fn linux_nav_preset() -> BindingSet {
    use pr4xis_domains::applied::hmi::input::keybindings::NamedKey;
    let mut bs = BindingSet::new("linux");
    let app = ModeId::new("app");
    let mut add = |mods: &[Modifier], key: Key, name: &str, desc: &str, cmd: &str| {
        bs.add(
            combo(mods, key),
            app.clone(),
            Action::new(name, desc, cmd),
            false,
        );
    };

    // Workspaces: Super+PageUp/PageDown switch; Super+Shift+PageUp/Down move window.
    add(
        &[Super],
        Key::Named(NamedKey::PageUp),
        "workspace_prev",
        "Previous workspace",
        "workspace, -1",
    );
    add(
        &[Super],
        Key::Named(NamedKey::PageDown),
        "workspace_next",
        "Next workspace",
        "workspace, +1",
    );
    add(
        &[Super, Shift],
        Key::Named(NamedKey::PageUp),
        "move_to_prev",
        "Move window ← workspace",
        "movetoworkspace, -1",
    );
    add(
        &[Super, Shift],
        Key::Named(NamedKey::PageDown),
        "move_to_next",
        "Move window → workspace",
        "movetoworkspace, +1",
    );

    // Window switch (Alt+Tab) + close (Alt+F4) — faithful.
    add(
        &[Alt],
        Key::Named(NamedKey::Tab),
        "switch_window",
        "Switch window",
        "cyclenext,",
    );
    add(
        &[Alt],
        Key::Function(4),
        "kill",
        "Close window",
        "killactive,",
    );

    // Maximize (Super+↑ → fullscreen) + tile (Super+←/→) + hide (Super+H) — adapted.
    add(
        &[Super],
        Key::Named(NamedKey::Up),
        "fullscreen",
        "Maximize window",
        "fullscreen",
    );
    add(
        &[Super],
        Key::Named(NamedKey::Left),
        "tile_left",
        "Tile window left",
        "movewindow, l",
    );
    add(
        &[Super],
        Key::Named(NamedKey::Right),
        "tile_right",
        "Tile window right",
        "movewindow, r",
    );
    add(
        &[Super],
        Key::Letter('h'),
        "hide",
        "Hide window",
        "movetoworkspacesilent, special",
    );
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
        assert_eq!(app.bindings["maximize"].action, "fullscreen");
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

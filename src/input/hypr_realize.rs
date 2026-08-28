//! Hyprland realization of praxis's abstract WM-action ontology.
//!
//! praxis owns the keybinding MODEL — [`WmAction`], [`ActionWord`], bindings,
//! modes, the engine — with no compositor vocabulary. The compositor-specific
//! rendering lives HERE, in the consumer: this module lowers each abstract
//! action to the legacy Hyprland action form (`"dispatcher, args"`), which is
//! the engine's canonical action string — the provider-aware IPC handle
//! (`hypr.rs`/`hypr_lua.rs`) translates it to `hl.dsp.*` at dispatch time when
//! the compositor runs the Lua config engine.
//!
//! The lowering mirrors the semantics praxis's ontology documents: Hyprland
//! capability gaps (container-tree focus, focus-layer toggle, stacking/default
//! layouts, the EWMH states without a user dispatcher) lower to the empty
//! word by design; both maximize axes collapse to `fullscreen, 1`; minimize is
//! emulated with a silent move to the `special:minimized` scratchpad; `Above`
//! and `Sticky` both lower to `pin` (Hyprland's pin is above+sticky).
//!
//! A multi-action word cannot ride one Hyprland bind (one dispatcher each), so
//! it is realized as a single `exec` replaying each action through
//! `vogix hypr dispatch` — dialect-correct on both config engines.

use pr4xis_domains::applied::hmi::input::window_state::{StateBit, StateDelta};
use pr4xis_domains::applied::hmi::input::wm_action::{
    ActionWord, Cycle, Direction, FocusBy, Follow, LayoutKind, OutputSel, SubmapTarget, WmAction,
    WorkspaceTarget,
};

/// The Hyprland direction token (`l`/`r`/`u`/`d`).
fn direction(d: Direction) -> char {
    match d {
        Direction::Left => 'l',
        Direction::Right => 'r',
        Direction::Up => 'u',
        Direction::Down => 'd',
    }
}

/// The Hyprland workspace argument.
fn workspace(w: &WorkspaceTarget) -> String {
    match w {
        WorkspaceTarget::Index(n) => n.to_string(),
        WorkspaceTarget::Relative(k) => {
            if *k >= 0 {
                format!("+{k}")
            } else {
                k.to_string()
            }
        }
        WorkspaceTarget::Named(s) => s.clone(),
        WorkspaceTarget::Last => "previous".to_string(),
        WorkspaceTarget::Special(s) if s.is_empty() => "special".to_string(),
        WorkspaceTarget::Special(s) => format!("special:{s}"),
    }
}

/// The Hyprland monitor argument.
fn output(s: &OutputSel) -> String {
    match s {
        OutputSel::Direction(d) => direction(*d).to_string(),
        OutputSel::Named(s) => s.clone(),
        OutputSel::Relative(k) => {
            if *k >= 0 {
                format!("+{k}")
            } else {
                k.to_string()
            }
        }
    }
}

/// Lower a window-state mutation. Hyprland exposes only TOGGLE dispatchers, so
/// the EWMH add/remove/toggle distinction is not observable here; states
/// outside Hyprland's capability lower to the empty word (intentional — a
/// Hyprland-targeting preset binds only in-capability actions).
fn lower_state(d: &StateDelta) -> Vec<String> {
    match d.bit {
        StateBit::Fullscreen => vec!["fullscreen".to_string()],
        StateBit::MaximizedVert | StateBit::MaximizedHorz => vec!["fullscreen, 1".to_string()],
        StateBit::Hidden => vec!["movetoworkspacesilent, special:minimized".to_string()],
        StateBit::Floating => vec!["togglefloating,".to_string()],
        StateBit::Above | StateBit::Sticky => vec!["pin,".to_string()],
        StateBit::PseudoTiled => vec!["pseudo,".to_string()],
        StateBit::Shaded
        | StateBit::Below
        | StateBit::SkipTaskbar
        | StateBit::SkipPager
        | StateBit::Modal
        | StateBit::DemandsAttention
        | StateBit::Focused => Vec::new(),
    }
}

/// Lower one abstract action to its legacy Hyprland action string(s).
pub fn lower(action: &WmAction) -> Vec<String> {
    match action {
        WmAction::Focus(FocusBy::Direction(d)) => vec![format!("movefocus, {}", direction(*d))],
        WmAction::Focus(FocusBy::Cycle(Cycle::Forward)) => vec!["cyclenext,".to_string()],
        WmAction::Focus(FocusBy::Cycle(Cycle::Backward)) => vec!["cyclenext, prev".to_string()],
        // No container tree and no tiling/floating focus-layer toggle in
        // Hyprland — capability gaps, empty word.
        WmAction::Focus(FocusBy::Tree(_)) | WmAction::Focus(FocusBy::Layer) => Vec::new(),
        WmAction::MoveWindow(d) => vec![format!("movewindow, {}", direction(*d))],
        WmAction::SwapWindow(d) => vec![format!("swapwindow, {}", direction(*d))],
        WmAction::Resize(d, amt) => {
            let a = *amt as i16;
            let (dx, dy) = match d {
                Direction::Left => (-a, 0),
                Direction::Right => (a, 0),
                Direction::Up => (0, -a),
                Direction::Down => (0, a),
            };
            vec![format!("resizeactive, {dx} {dy}")]
        }
        WmAction::Close => vec!["killactive,".to_string()],
        WmAction::State(d) => lower_state(d),
        // Only the split toggle exists; all orientations collapse to it.
        WmAction::Split(_) => vec!["layoutmsg, togglesplit".to_string()],
        WmAction::ToggleGroup => vec!["togglegroup,".to_string()],
        // Tabbed realizes via window groups; stacking/default have no dispatcher.
        WmAction::Layout(LayoutKind::Tabbed) => vec!["togglegroup,".to_string()],
        WmAction::Layout(LayoutKind::Stacking | LayoutKind::Default) => Vec::new(),
        WmAction::CycleGroup(Cycle::Forward) => vec!["changegroupactive, f".to_string()],
        WmAction::CycleGroup(Cycle::Backward) => vec!["changegroupactive, b".to_string()],
        WmAction::Workspace(w) => vec![format!("workspace, {}", workspace(w))],
        WmAction::MoveToWorkspace(w, Follow::Follow) => {
            vec![format!("movetoworkspace, {}", workspace(w))]
        }
        WmAction::MoveToWorkspace(w, Follow::Silent) => {
            vec![format!("movetoworkspacesilent, {}", workspace(w))]
        }
        WmAction::RenameWorkspace(w, name) => {
            vec![format!("renameworkspace, {} {name}", workspace(w))]
        }
        WmAction::ToggleSpecialWorkspace(n) => vec![format!("togglespecialworkspace, {n}")],
        WmAction::FocusMonitor(s) => vec![format!("focusmonitor, {}", output(s))],
        WmAction::MoveToMonitor(s) => vec![format!("movewindow, mon:{}", output(s))],
        WmAction::Exec(cmd) => vec![format!("exec, {cmd}")],
        WmAction::Submap(SubmapTarget::Enter(m)) => vec![format!("submap, {}", m.0)],
        WmAction::Submap(SubmapTarget::Reset) => vec!["submap, reset".to_string()],
    }
}

/// Realize an action word as the command string a *single* keybind emits.
///
/// One action → its action string directly. A composite is batched into one
/// `exec` whose parts replay through `vogix hypr dispatch` (a Hyprland bind
/// carries exactly one dispatcher); the CLI verb speaks whichever IPC dialect
/// the compositor runs, so the composite survives the Lua-engine flip. A
/// capability-gap word realizes to the empty string.
pub fn realize_word(word: &ActionWord) -> String {
    let parts: Vec<String> = word.0.iter().flat_map(lower).collect();
    match parts.as_slice() {
        [] => String::new(),
        [one] => one.clone(),
        many => {
            let replayed: Vec<String> = many
                .iter()
                // POSIX single-quote escaping, so an embedded quote in an
                // Exec command cannot break out of the argument.
                .map(|p| format!("vogix hypr dispatch '{}'", p.replace('\'', r"'\''")))
                .collect();
            format!("exec, {}", replayed.join(" ; "))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pr4xis_domains::applied::hmi::input::wm_action::realize as praxis_realize;

    #[test]
    fn float_pin_composite_replays_through_vogix_hypr_dispatch() {
        let word = ActionWord(vec![WmAction::toggle_float(), WmAction::pin()]);
        assert_eq!(
            realize_word(&word),
            "exec, vogix hypr dispatch 'togglefloating,' ; vogix hypr dispatch 'pin,'"
        );
    }

    #[test]
    fn single_action_words_render_directly() {
        let cases = [
            (WmAction::focus(Direction::Left), "movefocus, l"),
            (WmAction::Close, "killactive,"),
            (WmAction::toggle_split(), "layoutmsg, togglesplit"),
            (
                WmAction::Workspace(WorkspaceTarget::Index(3)),
                "workspace, 3",
            ),
            (
                WmAction::MoveToWorkspace(WorkspaceTarget::Index(2), Follow::Silent),
                "movetoworkspacesilent, 2",
            ),
            (
                WmAction::ToggleSpecialWorkspace("console".to_string()),
                "togglespecialworkspace, console",
            ),
            (WmAction::maximize(), "fullscreen, 1"),
            (WmAction::fullscreen(), "fullscreen"),
        ];
        for (action, expected) in cases {
            assert_eq!(
                realize_word(&ActionWord(vec![action.clone()])),
                expected,
                "for {action:?}"
            );
        }
    }

    // The vogix lowering must stay byte-identical to praxis's own Hyprland
    // realization for every single-dispatcher representative action — praxis
    // remains the documented ontology; vogix owns the wire form. (Composites
    // diverge by design: praxis's legacy exec-batch is not dialect-aware.)
    #[test]
    fn single_dispatch_lowering_matches_praxis_realization() {
        for action in WmAction::representative_actions() {
            let ours = lower(&action);
            if ours.len() == 1 {
                assert_eq!(
                    ours[0],
                    praxis_realize(&action),
                    "vogix and praxis lowerings drifted for {action:?}"
                );
            }
        }
    }

    #[test]
    fn capability_gaps_realize_to_the_empty_string() {
        assert_eq!(
            realize_word(&ActionWord(vec![WmAction::focus_parent()])),
            ""
        );
    }
}

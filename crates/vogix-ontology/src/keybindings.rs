/// Keybinding ontology — formal model of keyboard shortcuts and presets.
///
/// A keybinding maps (Key, Modifiers, Mode) → Action.
/// The ontology defines the structure; presets (vim, emacs, macOS, windows)
/// are instances — different morphism sets over the same key space.
///
/// Sources:
/// - Card, Mackinlay & Robertson, "Morphological Analysis of Input Devices" (1991)
/// - Beaudouin-Lafon, "Instrumental Interaction" (2000): modes activate instruments
/// - Harel, "Statecharts" (1987): mode-scoped keybindings
/// - XKB specification: modifier model (Shift, Ctrl, Alt, Super, Hyper)
use crate::modes::ModeId;
use praxis::ontology::Axiom;
use std::collections::{HashMap, HashSet};

/// A physical key identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Key {
    Letter(char),           // a-z
    Number(u8),             // 0-9
    Function(u8),           // F1-F24
    Named(NamedKey),        // Enter, Escape, Space, Tab, etc.
    Mouse(MouseButton),     // mouse buttons
}

/// Named (non-character) keys.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum NamedKey {
    Enter,
    Escape,
    Space,
    Tab,
    Backspace,
    Delete,
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
    PageUp,
    PageDown,
    Print,
    ScrollLock,
    CapsLock,
    /// Media/hardware keys
    VolumeUp,
    VolumeDown,
    VolumeMute,
    BrightnessUp,
    BrightnessDown,
    MediaPlay,
    MediaNext,
    MediaPrev,
}

/// Mouse buttons.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    ScrollUp,
    ScrollDown,
}

/// Modifier keys — can be combined.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Modifier {
    Shift,
    Ctrl,
    Alt,
    Super,
    Hyper,
}

/// A key combination: modifiers + key.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct KeyCombo {
    pub modifiers: Vec<Modifier>,
    pub key: Key,
}

impl KeyCombo {
    pub fn new(key: Key) -> Self {
        Self {
            modifiers: Vec::new(),
            key,
        }
    }

    pub fn with_mod(mut self, modifier: Modifier) -> Self {
        if !self.modifiers.contains(&modifier) {
            self.modifiers.push(modifier);
            self.modifiers.sort(); // canonical order
        }
        self
    }

    /// Human-readable representation.
    pub fn display(&self) -> String {
        let mut parts: Vec<String> = self
            .modifiers
            .iter()
            .map(|m| match m {
                Modifier::Shift => "Shift",
                Modifier::Ctrl => "Ctrl",
                Modifier::Alt => "Alt",
                Modifier::Super => "Super",
                Modifier::Hyper => "Hyper",
            })
            .map(String::from)
            .collect();
        parts.push(match &self.key {
            Key::Letter(c) => c.to_uppercase().to_string(),
            Key::Number(n) => n.to_string(),
            Key::Function(n) => format!("F{}", n),
            Key::Named(k) => format!("{:?}", k),
            Key::Mouse(b) => format!("Mouse{:?}", b),
        });
        parts.join(" + ")
    }
}

/// An action that a keybinding triggers.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Action {
    pub name: String,
    pub description: String,
    /// The action command (e.g., "killactive,", "exec, $TERMINAL")
    pub command: String,
}

impl Action {
    pub fn new(name: impl Into<String>, desc: impl Into<String>, cmd: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: desc.into(),
            command: cmd.into(),
        }
    }
}

/// A keybinding: key combo + mode context → action.
#[derive(Debug, Clone)]
pub struct Binding {
    pub combo: KeyCombo,
    pub mode: ModeId,
    pub action: Action,
    /// Does this binding repeat when held?
    pub repeat: bool,
}

/// A keybinding set — all bindings for a configuration.
#[derive(Debug, Clone)]
pub struct BindingSet {
    pub name: String,
    pub bindings: Vec<Binding>,
}

impl BindingSet {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            bindings: Vec::new(),
        }
    }

    pub fn add(&mut self, combo: KeyCombo, mode: ModeId, action: Action, repeat: bool) {
        self.bindings.push(Binding {
            combo,
            mode,
            action,
            repeat,
        });
    }

    /// Get all bindings for a specific mode.
    pub fn for_mode(&self, mode: &ModeId) -> Vec<&Binding> {
        self.bindings.iter().filter(|b| b.mode == *mode).collect()
    }

    /// Detect conflicts: same key combo in the same mode.
    pub fn conflicts(&self) -> Vec<(&Binding, &Binding)> {
        let mut found = Vec::new();
        for (i, a) in self.bindings.iter().enumerate() {
            for b in self.bindings.iter().skip(i + 1) {
                if a.combo == b.combo && a.mode == b.mode {
                    found.push((a, b));
                }
            }
        }
        found
    }

    /// Count of unique key combos per mode.
    pub fn combos_per_mode(&self) -> HashMap<ModeId, usize> {
        let mut counts: HashMap<ModeId, HashSet<&KeyCombo>> = HashMap::new();
        for b in &self.bindings {
            counts.entry(b.mode.clone()).or_default().insert(&b.combo);
        }
        counts.into_iter().map(|(k, v)| (k, v.len())).collect()
    }
}

/// A keybinding remap: transforms one key combo to another.
/// Used for Super→Ctrl remapping (macOS-style).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Remap {
    pub from: KeyCombo,
    pub to: KeyCombo,
}

/// A remap set — a collection of key remappings.
/// This is a functor: maps the key space to itself.
#[derive(Debug, Clone)]
pub struct RemapSet {
    pub name: String,
    pub remaps: Vec<Remap>,
}

impl RemapSet {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            remaps: Vec::new(),
        }
    }

    pub fn add(&mut self, from: KeyCombo, to: KeyCombo) {
        self.remaps.push(Remap { from, to });
    }

    /// Apply the remap: if the combo matches a `from`, return the `to`.
    pub fn apply(&self, combo: &KeyCombo) -> Option<&KeyCombo> {
        self.remaps
            .iter()
            .find(|r| r.from == *combo)
            .map(|r| &r.to)
    }
}

// ── Presets ──

/// macOS-style Super→Ctrl remap for common shortcuts.
///
/// Source: macOS uses Command (≈ Super) for copy/paste/save/etc.
/// On Linux, these are Ctrl+C/V/S. The remap makes Super behave like Command.
pub fn macos_remap() -> RemapSet {
    let mut rs = RemapSet::new("macos");
    let letters = "cvxzsafwtnprobluigdyq";
    for c in letters.chars() {
        rs.add(
            KeyCombo::new(Key::Letter(c)).with_mod(Modifier::Super),
            KeyCombo::new(Key::Letter(c)).with_mod(Modifier::Ctrl),
        );
    }
    rs
}

/// vim-style mode keybindings (insert returns to normal on Escape).
pub fn vim_preset() -> BindingSet {
    let mut bs = BindingSet::new("vim");
    let normal = ModeId::new("normal");
    let insert = ModeId::new("insert");

    // Normal mode: hjkl navigation, i=insert, :=command
    bs.add(
        KeyCombo::new(Key::Letter('h')),
        normal.clone(),
        Action::new("move_left", "Move cursor left", "movefocus, l"),
        false,
    );
    bs.add(
        KeyCombo::new(Key::Letter('j')),
        normal.clone(),
        Action::new("move_down", "Move cursor down", "movefocus, d"),
        false,
    );
    bs.add(
        KeyCombo::new(Key::Letter('k')),
        normal.clone(),
        Action::new("move_up", "Move cursor up", "movefocus, u"),
        false,
    );
    bs.add(
        KeyCombo::new(Key::Letter('l')),
        normal.clone(),
        Action::new("move_right", "Move cursor right", "movefocus, r"),
        false,
    );
    bs.add(
        KeyCombo::new(Key::Letter('i')),
        normal.clone(),
        Action::new("enter_insert", "Enter insert mode", "submap, insert"),
        false,
    );

    // Insert mode: Escape returns to normal
    bs.add(
        KeyCombo::new(Key::Named(NamedKey::Escape)),
        insert,
        Action::new("exit_insert", "Return to normal mode", "submap, reset"),
        false,
    );

    bs
}

/// CUA/Windows-style keybindings — standard PC shortcuts.
///
/// Source: IBM CUA specification (1987), Microsoft Windows UX Guidelines
pub fn cua_preset() -> BindingSet {
    let mut bs = BindingSet::new("cua");
    let app = ModeId::new("app");

    let binds = [
        ('c', "copy", "Copy", "exec, wl-copy"),
        ('v', "paste", "Paste", "exec, wl-paste"),
        ('x', "cut", "Cut", "exec, wl-copy"),
        ('z', "undo", "Undo", "exec, undo"),
        ('s', "save", "Save", "exec, save"),
        ('a', "select_all", "Select all", "exec, select-all"),
        ('f', "find", "Find", "exec, find"),
        ('n', "new_window", "New window", "exec, new-window"),
        ('o', "open", "Open file", "exec, open"),
        ('p', "print", "Print", "exec, print"),
        ('w', "close_tab", "Close tab", "killactive,"),
        ('t', "new_tab", "New tab", "exec, new-tab"),
    ];
    for (c, name, desc, cmd) in binds {
        bs.add(
            KeyCombo::new(Key::Letter(c)).with_mod(Modifier::Ctrl),
            app.clone(),
            Action::new(name, desc, cmd),
            false,
        );
    }

    // Alt+F4 = quit
    bs.add(
        KeyCombo::new(Key::Function(4)).with_mod(Modifier::Alt),
        app.clone(),
        Action::new("quit", "Quit application", "killactive,"),
        false,
    );
    // Alt+Tab = switch window
    bs.add(
        KeyCombo::new(Key::Named(NamedKey::Tab)).with_mod(Modifier::Alt),
        app,
        Action::new("switch_window", "Switch window", "cyclenext,"),
        false,
    );

    bs
}

/// emacs-style keybindings — Ctrl/Meta prefix navigation.
///
/// Source: GNU Emacs Manual, readline conventions
pub fn emacs_preset() -> BindingSet {
    let mut bs = BindingSet::new("emacs");
    let app = ModeId::new("app");

    // C-a/e/k/y — line editing (readline)
    bs.add(KeyCombo::new(Key::Letter('a')).with_mod(Modifier::Ctrl), app.clone(),
        Action::new("line_start", "Beginning of line", "exec, line-start"), false);
    bs.add(KeyCombo::new(Key::Letter('e')).with_mod(Modifier::Ctrl), app.clone(),
        Action::new("line_end", "End of line", "exec, line-end"), false);
    bs.add(KeyCombo::new(Key::Letter('k')).with_mod(Modifier::Ctrl), app.clone(),
        Action::new("kill_line", "Kill to end of line", "exec, kill-line"), false);
    bs.add(KeyCombo::new(Key::Letter('y')).with_mod(Modifier::Ctrl), app.clone(),
        Action::new("yank", "Yank (paste)", "exec, yank"), false);

    // C-f/b/n/p — character/line movement
    bs.add(KeyCombo::new(Key::Letter('f')).with_mod(Modifier::Ctrl), app.clone(),
        Action::new("forward_char", "Forward one character", "movefocus, r"), false);
    bs.add(KeyCombo::new(Key::Letter('b')).with_mod(Modifier::Ctrl), app.clone(),
        Action::new("backward_char", "Backward one character", "movefocus, l"), false);
    bs.add(KeyCombo::new(Key::Letter('n')).with_mod(Modifier::Ctrl), app.clone(),
        Action::new("next_line", "Next line", "movefocus, d"), false);
    bs.add(KeyCombo::new(Key::Letter('p')).with_mod(Modifier::Ctrl), app.clone(),
        Action::new("prev_line", "Previous line", "movefocus, u"), false);

    // M-f/b — word movement
    bs.add(KeyCombo::new(Key::Letter('f')).with_mod(Modifier::Alt), app.clone(),
        Action::new("forward_word", "Forward one word", "exec, forward-word"), false);
    bs.add(KeyCombo::new(Key::Letter('b')).with_mod(Modifier::Alt), app.clone(),
        Action::new("backward_word", "Backward one word", "exec, backward-word"), false);

    // C-g — cancel
    bs.add(KeyCombo::new(Key::Letter('g')).with_mod(Modifier::Ctrl), app.clone(),
        Action::new("cancel", "Cancel / keyboard quit", "submap, reset"), false);

    // C-s/r — search
    bs.add(KeyCombo::new(Key::Letter('s')).with_mod(Modifier::Ctrl), app.clone(),
        Action::new("search_forward", "Incremental search forward", "exec, search-forward"), false);
    bs.add(KeyCombo::new(Key::Letter('r')).with_mod(Modifier::Ctrl), app,
        Action::new("search_backward", "Incremental search backward", "exec, search-backward"), false);

    bs
}

/// i3/sway tiling WM keybindings — Super + key for WM actions.
///
/// Source: i3 User's Guide, sway(5) man page
pub fn i3_preset() -> BindingSet {
    let mut bs = BindingSet::new("i3");
    let app = ModeId::new("app");
    let resize = ModeId::new("resize");

    // Window management
    bs.add(KeyCombo::new(Key::Named(NamedKey::Enter)).with_mod(Modifier::Super), app.clone(),
        Action::new("terminal", "Launch terminal", "exec, $TERMINAL"), false);
    bs.add(KeyCombo::new(Key::Letter('d')).with_mod(Modifier::Super), app.clone(),
        Action::new("launcher", "Application launcher", "exec, walker"), false);
    bs.add(KeyCombo::new(Key::Letter('q')).with_mod(Modifier::Super).with_mod(Modifier::Shift), app.clone(),
        Action::new("kill", "Kill focused window", "killactive,"), false);
    bs.add(KeyCombo::new(Key::Letter('f')).with_mod(Modifier::Super), app.clone(),
        Action::new("fullscreen", "Toggle fullscreen", "fullscreen"), false);

    // Focus: Super+hjkl
    for (c, dir, desc) in [('h', "l", "left"), ('j', "d", "down"), ('k', "u", "up"), ('l', "r", "right")] {
        bs.add(KeyCombo::new(Key::Letter(c)).with_mod(Modifier::Super), app.clone(),
            Action::new(format!("focus_{desc}"), format!("Focus {desc}"), format!("movefocus, {dir}")), false);
    }

    // Move: Super+Shift+hjkl
    for (c, dir, desc) in [('h', "l", "left"), ('j', "d", "down"), ('k', "u", "up"), ('l', "r", "right")] {
        bs.add(KeyCombo::new(Key::Letter(c)).with_mod(Modifier::Super).with_mod(Modifier::Shift), app.clone(),
            Action::new(format!("move_{desc}"), format!("Move window {desc}"), format!("movewindow, {dir}")), false);
    }

    // Workspaces: Super+1-9
    for i in 1u8..=9 {
        bs.add(KeyCombo::new(Key::Number(i)).with_mod(Modifier::Super), app.clone(),
            Action::new(format!("workspace_{i}"), format!("Workspace {i}"), format!("workspace, {i}")), false);
    }

    // Move to workspace: Super+Shift+1-9
    for i in 1u8..=9 {
        bs.add(KeyCombo::new(Key::Number(i)).with_mod(Modifier::Super).with_mod(Modifier::Shift), app.clone(),
            Action::new(format!("move_to_{i}"), format!("Move to workspace {i}"), format!("movetoworkspace, {i}")), false);
    }

    // Layout
    bs.add(KeyCombo::new(Key::Letter('v')).with_mod(Modifier::Super), app.clone(),
        Action::new("split_v", "Split vertical", "layoutmsg, togglesplit"), false);
    bs.add(KeyCombo::new(Key::Named(NamedKey::Space)).with_mod(Modifier::Super).with_mod(Modifier::Shift), app.clone(),
        Action::new("float", "Toggle floating", "togglefloating,"), false);

    // Resize mode
    bs.add(KeyCombo::new(Key::Letter('r')).with_mod(Modifier::Super), app,
        Action::new("enter_resize", "Enter resize mode", "submap, resize"), false);

    // Resize mode bindings
    for (c, action, desc) in [
        ('h', "resizeactive, -30 0", "Shrink width"),
        ('j', "resizeactive, 0 30", "Grow height"),
        ('k', "resizeactive, 0 -30", "Shrink height"),
        ('l', "resizeactive, 30 0", "Grow width"),
    ] {
        bs.add(KeyCombo::new(Key::Letter(c)), resize.clone(),
            Action::new(format!("resize_{c}"), desc, action), true);
    }
    bs.add(KeyCombo::new(Key::Named(NamedKey::Escape)), resize,
        Action::new("exit_resize", "Exit resize mode", "submap, reset"), false);

    bs
}

/// tmux-style keybindings — prefix key (Ctrl+B) then action key.
///
/// Source: tmux(1) man page, vi copy mode conventions
pub fn tmux_preset() -> BindingSet {
    let mut bs = BindingSet::new("tmux");
    let prefix = ModeId::new("tmux-prefix");

    // Window management
    bs.add(KeyCombo::new(Key::Letter('c')), prefix.clone(),
        Action::new("new_window", "Create new window", "exec, tmux new-window"), false);
    bs.add(KeyCombo::new(Key::Letter('n')), prefix.clone(),
        Action::new("next_window", "Next window", "exec, tmux next-window"), false);
    bs.add(KeyCombo::new(Key::Letter('p')), prefix.clone(),
        Action::new("prev_window", "Previous window", "exec, tmux previous-window"), false);
    bs.add(KeyCombo::new(Key::Letter('l')), prefix.clone(),
        Action::new("last_window", "Last window", "exec, tmux last-window"), false);

    // Window numbers
    for i in 0u8..=9 {
        bs.add(KeyCombo::new(Key::Number(i)), prefix.clone(),
            Action::new(format!("window_{i}"), format!("Switch to window {i}"), format!("exec, tmux select-window -t {i}")), false);
    }

    // Pane splitting
    bs.add(KeyCombo::new(Key::Letter('%')), prefix.clone(),
        Action::new("split_v", "Split pane vertical", "exec, tmux split-window -h"), false);
    bs.add(KeyCombo::new(Key::Letter('"')), prefix.clone(),
        Action::new("split_h", "Split pane horizontal", "exec, tmux split-window -v"), false);

    // Pane navigation (arrows — hjkl conflicts with window commands in default tmux)
    for (key, dir, name) in [
        (NamedKey::Left, "L", "left"),
        (NamedKey::Down, "D", "down"),
        (NamedKey::Up, "U", "up"),
        (NamedKey::Right, "R", "right"),
    ] {
        bs.add(KeyCombo::new(Key::Named(key)), prefix.clone(),
            Action::new(format!("pane_{name}"), format!("Focus pane {name}"), format!("exec, tmux select-pane -{dir}")), false);
    }

    // Session/pane management
    bs.add(KeyCombo::new(Key::Letter('d')), prefix.clone(),
        Action::new("detach", "Detach session", "exec, tmux detach"), false);
    bs.add(KeyCombo::new(Key::Letter('z')), prefix.clone(),
        Action::new("zoom", "Toggle pane zoom", "exec, tmux resize-pane -Z"), false);
    bs.add(KeyCombo::new(Key::Letter('x')), prefix.clone(),
        Action::new("kill_pane", "Kill pane", "exec, tmux kill-pane"), false);
    bs.add(KeyCombo::new(Key::Letter('?')), prefix,
        Action::new("help", "List keybindings", "exec, tmux list-keys"), false);

    bs
}

// ── Axioms ──

/// No binding conflicts: same key combo in the same mode must not have two actions.
pub struct NoConflicts {
    pub bindings: BindingSet,
}

impl Axiom for NoConflicts {
    fn description(&self) -> &str {
        "no duplicate key combos in the same mode"
    }
    fn holds(&self) -> bool {
        self.bindings.conflicts().is_empty()
    }
}

/// Remap is injective: each `from` maps to exactly one `to`.
pub struct RemapInjective {
    pub remaps: RemapSet,
}

impl Axiom for RemapInjective {
    fn description(&self) -> &str {
        "remap is injective (each source maps to one target)"
    }
    fn holds(&self) -> bool {
        let froms: Vec<&KeyCombo> = self.remaps.remaps.iter().map(|r| &r.from).collect();
        let unique: HashSet<&KeyCombo> = froms.iter().copied().collect();
        froms.len() == unique.len()
    }
}

/// Every mode in the binding set has at least one binding.
pub struct AllModesHaveBindings {
    pub bindings: BindingSet,
    pub modes: Vec<ModeId>,
}

impl Axiom for AllModesHaveBindings {
    fn description(&self) -> &str {
        "every mode has at least one keybinding"
    }
    fn holds(&self) -> bool {
        self.modes
            .iter()
            .all(|m| !self.bindings.for_mode(m).is_empty())
    }
}

/// Super→Ctrl remap covers all standard shortcuts (copy, paste, save, etc).
pub struct MacosRemapComplete {
    pub remaps: RemapSet,
}

impl Axiom for MacosRemapComplete {
    fn description(&self) -> &str {
        "macOS remap covers essential shortcuts (C, V, X, Z, S, A)"
    }
    fn holds(&self) -> bool {
        let essential = ['c', 'v', 'x', 'z', 's', 'a'];
        essential.iter().all(|&c| {
            let combo = KeyCombo::new(Key::Letter(c)).with_mod(Modifier::Super);
            self.remaps.apply(&combo).is_some()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── KeyCombo tests ──

    #[test]
    fn test_key_combo_display() {
        let combo = KeyCombo::new(Key::Letter('c')).with_mod(Modifier::Ctrl);
        assert_eq!(combo.display(), "Ctrl + C");
    }

    #[test]
    fn test_key_combo_multi_mod() {
        let combo = KeyCombo::new(Key::Letter('s'))
            .with_mod(Modifier::Ctrl)
            .with_mod(Modifier::Shift);
        assert_eq!(combo.display(), "Shift + Ctrl + S");
    }

    #[test]
    fn test_key_combo_no_duplicate_mods() {
        let combo = KeyCombo::new(Key::Letter('a'))
            .with_mod(Modifier::Ctrl)
            .with_mod(Modifier::Ctrl);
        assert_eq!(combo.modifiers.len(), 1);
    }

    #[test]
    fn test_key_combo_sorted_mods() {
        let combo = KeyCombo::new(Key::Letter('a'))
            .with_mod(Modifier::Super)
            .with_mod(Modifier::Alt)
            .with_mod(Modifier::Ctrl);
        // Should be sorted: Ctrl < Alt < Super (by Ord derive)
        assert_eq!(combo.modifiers, vec![Modifier::Ctrl, Modifier::Alt, Modifier::Super]);
    }

    // ── Preset tests ──

    #[test]
    fn test_macos_remap() {
        let rs = macos_remap();
        let copy = KeyCombo::new(Key::Letter('c')).with_mod(Modifier::Super);
        let result = rs.apply(&copy).unwrap();
        assert_eq!(result.modifiers, vec![Modifier::Ctrl]);
        assert_eq!(result.key, Key::Letter('c'));
    }

    #[test]
    fn test_macos_remap_complete() {
        let rs = macos_remap();
        assert!(MacosRemapComplete { remaps: rs }.holds());
    }

    #[test]
    fn test_macos_remap_injective() {
        let rs = macos_remap();
        assert!(RemapInjective { remaps: rs }.holds());
    }

    #[test]
    fn test_vim_preset_no_conflicts() {
        let bs = vim_preset();
        assert!(NoConflicts { bindings: bs }.holds());
    }

    #[test]
    fn test_vim_preset_has_hjkl() {
        let bs = vim_preset();
        let normal = ModeId::new("normal");
        let normal_bindings = bs.for_mode(&normal);
        let keys: Vec<_> = normal_bindings.iter().map(|b| &b.combo.key).collect();
        assert!(keys.contains(&&Key::Letter('h')));
        assert!(keys.contains(&&Key::Letter('j')));
        assert!(keys.contains(&&Key::Letter('k')));
        assert!(keys.contains(&&Key::Letter('l')));
    }

    #[test]
    fn test_vim_preset_modes_have_bindings() {
        let bs = vim_preset();
        let modes = vec![ModeId::new("normal"), ModeId::new("insert")];
        assert!(AllModesHaveBindings { bindings: bs, modes }.holds());
    }

    // ── Conflict detection ──

    #[test]
    fn test_conflict_detected() {
        let mut bs = BindingSet::new("conflicting");
        let mode = ModeId::new("test");
        let combo = KeyCombo::new(Key::Letter('a')).with_mod(Modifier::Ctrl);
        bs.add(
            combo.clone(),
            mode.clone(),
            Action::new("a1", "Action 1", "cmd1"),
            false,
        );
        bs.add(
            combo,
            mode,
            Action::new("a2", "Action 2", "cmd2"),
            false,
        );
        assert!(!NoConflicts { bindings: bs.clone() }.holds());
        assert_eq!(bs.conflicts().len(), 1);
    }

    #[test]
    fn test_same_key_different_mode_no_conflict() {
        let mut bs = BindingSet::new("multi-mode");
        let combo = KeyCombo::new(Key::Letter('a'));
        bs.add(
            combo.clone(),
            ModeId::new("mode1"),
            Action::new("a1", "Action 1", "cmd1"),
            false,
        );
        bs.add(
            combo,
            ModeId::new("mode2"),
            Action::new("a2", "Action 2", "cmd2"),
            false,
        );
        assert!(NoConflicts { bindings: bs }.holds());
    }

    // ── CUA preset ──

    #[test]
    fn test_cua_preset_no_conflicts() {
        assert!(NoConflicts { bindings: cua_preset() }.holds());
    }

    #[test]
    fn test_cua_has_copy_paste() {
        let bs = cua_preset();
        let app = ModeId::new("app");
        let bindings = bs.for_mode(&app);
        let names: Vec<_> = bindings.iter().map(|b| b.action.name.as_str()).collect();
        assert!(names.contains(&"copy"));
        assert!(names.contains(&"paste"));
        assert!(names.contains(&"cut"));
    }

    // ── emacs preset ──

    #[test]
    fn test_emacs_preset_no_conflicts() {
        assert!(NoConflicts { bindings: emacs_preset() }.holds());
    }

    #[test]
    fn test_emacs_has_readline() {
        let bs = emacs_preset();
        let app = ModeId::new("app");
        let bindings = bs.for_mode(&app);
        let names: Vec<_> = bindings.iter().map(|b| b.action.name.as_str()).collect();
        assert!(names.contains(&"line_start"));  // C-a
        assert!(names.contains(&"line_end"));    // C-e
        assert!(names.contains(&"kill_line"));   // C-k
        assert!(names.contains(&"yank"));        // C-y
    }

    // ── i3 preset ──

    #[test]
    fn test_i3_preset_no_conflicts() {
        assert!(NoConflicts { bindings: i3_preset() }.holds());
    }

    #[test]
    fn test_i3_has_workspaces() {
        let bs = i3_preset();
        let app = ModeId::new("app");
        let bindings = bs.for_mode(&app);
        let names: Vec<_> = bindings.iter().map(|b| b.action.name.as_str()).collect();
        for i in 1..=9 {
            assert!(names.contains(&format!("workspace_{i}").as_str()), "missing workspace {i}");
        }
    }

    #[test]
    fn test_i3_has_hjkl_focus() {
        let bs = i3_preset();
        let app = ModeId::new("app");
        let bindings = bs.for_mode(&app);
        let names: Vec<_> = bindings.iter().map(|b| b.action.name.as_str()).collect();
        assert!(names.contains(&"focus_left"));
        assert!(names.contains(&"focus_right"));
        assert!(names.contains(&"focus_up"));
        assert!(names.contains(&"focus_down"));
    }

    #[test]
    fn test_i3_resize_mode() {
        let bs = i3_preset();
        let resize = ModeId::new("resize");
        let bindings = bs.for_mode(&resize);
        assert!(bindings.len() >= 5, "resize mode should have hjkl + escape");
    }

    // ── tmux preset ──

    #[test]
    fn test_tmux_preset_no_conflicts() {
        assert!(NoConflicts { bindings: tmux_preset() }.holds());
    }

    #[test]
    fn test_tmux_has_window_management() {
        let bs = tmux_preset();
        let prefix = ModeId::new("tmux-prefix");
        let bindings = bs.for_mode(&prefix);
        let names: Vec<_> = bindings.iter().map(|b| b.action.name.as_str()).collect();
        assert!(names.contains(&"new_window"));
        assert!(names.contains(&"next_window"));
        assert!(names.contains(&"prev_window"));
        assert!(names.contains(&"detach"));
    }

    #[test]
    fn test_tmux_has_pane_navigation() {
        let bs = tmux_preset();
        let prefix = ModeId::new("tmux-prefix");
        let bindings = bs.for_mode(&prefix);
        let names: Vec<_> = bindings.iter().map(|b| b.action.name.as_str()).collect();
        assert!(names.contains(&"pane_left"));
        assert!(names.contains(&"pane_down"));
        assert!(names.contains(&"pane_up"));
        assert!(names.contains(&"pane_right"));
    }

    // ── Cross-preset tests ──

    #[test]
    fn test_all_presets_no_conflicts() {
        for (name, bs) in [
            ("vim", vim_preset()),
            ("cua", cua_preset()),
            ("emacs", emacs_preset()),
            ("i3", i3_preset()),
            ("tmux", tmux_preset()),
        ] {
            let axiom = NoConflicts { bindings: bs };
            assert!(axiom.holds(), "{name} preset has conflicts");
        }
    }

    // ── Property-based tests ──
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn prop_macos_remap_always_ctrl(idx in 0u8..26) {
            let c = (b'a' + idx) as char;
            let rs = macos_remap();
            let combo = KeyCombo::new(Key::Letter(c)).with_mod(Modifier::Super);
            if let Some(result) = rs.apply(&combo) {
                prop_assert!(result.modifiers == vec![Modifier::Ctrl]);
                prop_assert!(result.key == Key::Letter(c));
            }
        }

        #[test]
        fn prop_remap_preserves_key(idx in 0u8..26) {
            let c = (b'a' + idx) as char;
            let rs = macos_remap();
            let combo = KeyCombo::new(Key::Letter(c)).with_mod(Modifier::Super);
            if let Some(result) = rs.apply(&combo) {
                prop_assert!(result.key == combo.key);
            }
        }

        #[test]
        fn prop_display_contains_key(idx in 0u8..26) {
            let c = (b'a' + idx) as char;
            let combo = KeyCombo::new(Key::Letter(c));
            let display = combo.display();
            prop_assert!(display.contains(c.to_uppercase().next().unwrap()));
        }

        #[test]
        fn prop_no_duplicate_mods_after_double_add(idx in 0u8..26) {
            let c = (b'a' + idx) as char;
            let combo = KeyCombo::new(Key::Letter(c))
                .with_mod(Modifier::Ctrl)
                .with_mod(Modifier::Ctrl)
                .with_mod(Modifier::Ctrl);
            prop_assert_eq!(combo.modifiers.len(), 1);
        }

        #[test]
        fn prop_mods_always_sorted(idx in 0u8..26) {
            let c = (b'a' + idx) as char;
            let combo = KeyCombo::new(Key::Letter(c))
                .with_mod(Modifier::Hyper)
                .with_mod(Modifier::Shift)
                .with_mod(Modifier::Alt)
                .with_mod(Modifier::Ctrl)
                .with_mod(Modifier::Super);
            let mods = &combo.modifiers;
            for w in mods.windows(2) {
                prop_assert!(w[0] <= w[1], "modifiers not sorted");
            }
        }

        #[test]
        fn prop_binding_set_no_conflicts_when_unique_keys(n in 1usize..10) {
            let mut bs = BindingSet::new("unique");
            let mode = ModeId::new("test");
            for i in 0..n.min(26) {
                let c = (b'a' + i as u8) as char;
                bs.add(
                    KeyCombo::new(Key::Letter(c)),
                    mode.clone(),
                    Action::new(format!("a{}", i), "test", "cmd"),
                    false,
                );
            }
            let axiom = NoConflicts { bindings: bs };
            prop_assert!(axiom.holds());
        }
    }
}

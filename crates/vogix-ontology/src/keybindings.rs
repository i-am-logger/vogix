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

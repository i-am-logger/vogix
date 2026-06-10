//! The device layer: the keystroke router and the evdev grab/uinput loop.
//!
//! [`Router`] is the brain and is pure: feed it `(code, value, now)` and it
//! returns [`Effect`]s (emit a key, dispatch a Hyprland action, note a mode
//! change). It interprets the *loaded* schema — the engine, the tap/hold
//! detector, and the binding tables are all driven by [`super::schema`], nothing
//! is hard-coded. [`run`] is the thin I/O shell: it grabs the real keyboard,
//! creates a uinput device, and pumps events through the router.
//!
//! Modifier policy (the one pragmatic simplification): the mod key (Super) is
//! swallowed — vogix owns every Super-combo (remaps + app bindings). Shift/Ctrl/
//! Alt pass through (so typing and app shortcuts work) and are tracked for chord
//! matching. A bound chord swallows its *base* key; in catchall (`submap`) modes
//! every unbound key is swallowed too; in passthrough modes unbound keys are
//! re-emitted. CapsLock is never emitted — it is the mode trigger.

use super::health;
use super::keys::{
    Chord, Modifier, Mods, is_capslock, key_to_keycode, keycode_to_key, modifier_code, modifier_of,
    modifiers_to_mods, parse_chord,
};
use super::schema::{ActionKind, Schema, parse_action};
use super::taphold::{CapsDetector, CapsEvent, CapsIntent};
use evdev::KeyCode;
use pr4xis::engine::Engine;
use pr4xis_domains::applied::hmi::input::engine::{ModeTransition, drive, new_input_engine};
use pr4xis_domains::applied::hmi::input::keybindings::{
    Key, KeyCombo, Modifier as PxMod, RemapSet,
};
use pr4xis_domains::applied::hmi::input::modes::ModeId;
use std::collections::{HashMap, HashSet};

/// The name of our virtual re-emit device. We must never grab it back, or the
/// engine would capture its own emitted events; the device filter
/// ([`super::devfilter`]) shares this exact exclusion.
pub(crate) const VIRTUAL_NAME: &str = "vogix-input";

/// A side effect the I/O shell should perform.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Effect {
    /// Emit a key event on the virtual device (passthrough or synthesized).
    Emit { code: u16, value: i32 },
    /// Run a Hyprland action string over the control socket.
    Dispatch(String),
    /// Set a Hyprland keyword over the control socket (`keyword <key> <value>`),
    /// e.g. a per-mode border colour — the mode-visibility surface.
    Keyword { key: String, value: String },
    /// The active mode changed (for visuals / logging / the waybar surface).
    ModeChanged(String),
}

/// One grabbed keyboard the engine owns: the evdev device, its `/dev/input`
/// node (to dedup inotify add events), and its observability identity +
/// counters. Held in a stable `Vec<Option<Grabbed>>` slot — a slot is `None`d
/// when its device unplugs (so the counter indices never shift) and a
/// hotplugged keyboard reuses a free slot or pushes a new one.
struct Grabbed {
    dev: evdev::Device,
    node: std::path::PathBuf,
    meta: health::DeviceMeta,
    counters: health::Counters,
}

/// A binding resolved from the schema for fast lookup.
#[derive(Debug, Clone)]
struct Resolved {
    action: ActionKind,
    exit_after: bool,
    repeat: bool,
}

/// The pure keystroke router — interprets the loaded schema.
pub struct Router {
    /// `Option` only so we can move through the move-based `Engine::next`.
    engine: Option<Engine<ModeTransition>>,
    detector: CapsDetector,
    mods: Mods,
    /// mode name → (chord → binding).
    bindings: HashMap<String, HashMap<Chord, Resolved>>,
    /// mode name → catchall (swallows unbound keys).
    catchall: HashMap<String, bool>,
    /// The paradigm's Super-modifier remap set (e.g. macOS-Command: Super+C →
    /// Ctrl+C), a praxis `RemapSet` consulted via `apply`.
    remaps: RemapSet,
    /// Window classes that are terminals — the remap is context-adjusted there.
    terminal_classes: HashSet<String>,
    /// The focused window's class (fed by the I/O shell from Hyprland's
    /// active-window event stream); drives the terminal-aware remap policy.
    active_class: Option<String>,
    /// Per-mode border colours (active, inactive) for the visibility surface.
    mode_colors: HashMap<String, (String, String)>,
    /// Timestamp (ms) of the last input event — drives sticky-mode idle revert.
    last_activity: u64,
    /// Idle threshold (ms) after which a sticky mode auto-reverts (loaded).
    sticky_idle_ms: u64,
    caps_target: Option<ModeId>,
    root: ModeId,
}

impl Router {
    /// Build a router from the loaded schema.
    pub fn new(schema: &Schema) -> Self {
        let graph = schema.build_mode_graph();
        let root = graph.root.clone();
        // "Catchall" (does a mode swallow unbound keys?) is a praxis
        // ModeProperties quality — read it from the built graph, the single
        // source of truth, rather than recomputing kind=="submap" here.
        let catchall: HashMap<String, bool> = graph
            .modes
            .iter()
            .map(|(id, props)| (id.0.clone(), props.catchall))
            .collect();
        let engine = Some(new_input_engine(graph));

        let mut bindings = HashMap::new();
        for (mode_name, mode_spec) in &schema.modes {
            let mut map = HashMap::new();
            for b in mode_spec.bindings.values() {
                if let Some(chord) = parse_chord(&b.key) {
                    map.insert(
                        chord,
                        Resolved {
                            action: parse_action(&b.action),
                            exit_after: b.exit_after,
                            repeat: b.repeat,
                        },
                    );
                }
            }
            // Esc safety-net: a mode's `exit` key always returns to root, so the
            // user can never be stranded (Harel reachable-default; the engine's
            // always-legal ExitToRoot). Synthesized ONLY for catchall (submap)
            // modes — in normal/passthrough modes (app, console) the exit key
            // must reach the focused application (Esc in vim/dialogs/tmux), so we
            // never swallow it there. An explicit binding for the same key wins.
            if *catchall.get(mode_name).unwrap_or(&false)
                && let Some(chord) = mode_spec.exit.as_deref().and_then(parse_chord)
            {
                map.entry(chord).or_insert(Resolved {
                    action: ActionKind::Submap("reset".to_string()),
                    exit_after: false,
                    repeat: false,
                });
            }
            bindings.insert(mode_name.clone(), map);
        }

        // The remap set comes from the loaded interaction paradigm (a praxis
        // preset, e.g. macos_remap) — cited + axiom-checkable, not a hand table.
        let remaps = schema.remap_set();

        Self {
            engine,
            detector: CapsDetector::new(schema.tap_hold_ms()),
            mods: Mods::default(),
            bindings,
            catchall,
            remaps,
            terminal_classes: schema.terminal_classes.iter().cloned().collect(),
            active_class: None,
            mode_colors: schema
                .mode_colors
                .iter()
                .map(|(m, c)| (m.clone(), (c.active.clone(), c.inactive.clone())))
                .collect(),
            last_activity: 0,
            sticky_idle_ms: schema.sticky_idle_ms(),
            caps_target: schema.caps_target().map(ModeId::new),
            root,
        }
    }

    /// True when a STICKY (locked) mode is active — the only kind that idle-reverts.
    fn sticky_active(&self) -> bool {
        self.engine
            .as_ref()
            .expect("engine present")
            .situation()
            .sticky
    }

    /// When (in the event clock) a sticky mode should auto-revert if still idle.
    /// `None` unless a sticky mode is active. The poll loop folds this into its
    /// wake-up deadline.
    pub fn idle_deadline(&self) -> Option<u64> {
        self.sticky_active()
            .then(|| self.last_activity + self.sticky_idle_ms)
    }

    /// Auto-revert a sticky mode to root once it has been idle past the
    /// threshold (a forgotten sticky mode self-heals). No-op otherwise.
    pub fn on_idle(&mut self, now: u64) -> Vec<Effect> {
        let mut fx = Vec::new();
        if self.sticky_active() && now.saturating_sub(self.last_activity) >= self.sticky_idle_ms {
            self.apply_and_note(ModeTransition::ExitToRoot, &mut fx);
        }
        fx
    }

    /// Update the focused-window class (fed by the I/O shell from the Hyprland
    /// active-window event stream). Drives the terminal-aware Super→Ctrl policy.
    pub fn set_active_class(&mut self, class: Option<String>) {
        self.active_class = class;
    }

    /// True when the focused window is a known terminal class.
    fn active_is_terminal(&self) -> bool {
        self.active_class
            .as_ref()
            .is_some_and(|c| self.terminal_classes.contains(c))
    }

    /// The current mode name.
    pub fn mode(&self) -> String {
        self.engine
            .as_ref()
            .expect("engine present")
            .situation()
            .mode
            .0
            .clone()
    }

    /// The detector's next poll deadline (ms), if a caps gesture is pending.
    pub fn deadline(&self) -> Option<u64> {
        self.detector.deadline()
    }

    fn at_root(&self) -> bool {
        self.mode() == self.root.0
    }

    fn apply(&mut self, action: ModeTransition) {
        let e = self.engine.take().expect("engine present");
        self.engine = Some(drive(e, action));
    }

    /// Apply a transition and push `ModeChanged` ONLY when the mode actually
    /// changed. Without this guard a no-op transition (e.g. `ReleaseHold` while
    /// already at root, or a tap that exits a mode the engine has already left)
    /// still emitted a `ModeChanged` — which is what produced the spurious
    /// double `mode → app` lines seen in the journal.
    fn apply_and_note(&mut self, action: ModeTransition, fx: &mut Vec<Effect>) {
        let from = self.mode();
        self.apply(action);
        let to = self.mode();
        if from != to {
            fx.push(Effect::ModeChanged(to.clone()));
            self.push_border(&to, fx);
        }
    }

    /// Mode-visibility surface: paint the active-window border for `mode` so the
    /// user can always SEE the current mode (the cure for mode error — Norman
    /// 1981). Colours are loaded data (theme-derived); a mode with no colour
    /// simply isn't painted.
    fn push_border(&self, mode: &str, fx: &mut Vec<Effect>) {
        if let Some((active, inactive)) = self.mode_colors.get(mode) {
            fx.push(Effect::Keyword {
                key: "general:col.active_border".into(),
                value: active.clone(),
            });
            fx.push(Effect::Keyword {
                key: "general:col.inactive_border".into(),
                value: inactive.clone(),
            });
        }
    }

    /// Border effects for the CURRENT mode — painted once at startup so the
    /// visibility cue is correct from the start (a prior session may have left a
    /// non-app border on the compositor across an engine restart).
    pub fn paint_current_mode(&self) -> Vec<Effect> {
        let mut fx = Vec::new();
        self.push_border(&self.mode(), &mut fx);
        fx
    }

    /// Poll deadline elapsed — let a pure caps-hold resolve.
    pub fn on_timeout(&mut self, now: u64) -> Vec<Effect> {
        let mut fx = Vec::new();
        if let Some(CapsIntent::HoldStart) = self.detector.feed(CapsEvent::Timeout(now)) {
            self.enter_momentary(&mut fx);
        }
        fx
    }

    /// Process one key event; returns the effects to perform.
    pub fn on_key(&mut self, code: u16, value: i32, now: u64) -> Vec<Effect> {
        let key = KeyCode(code);
        let mut fx = Vec::new();
        self.last_activity = now; // any input resets the sticky-idle timer

        // CapsLock — the mode trigger; never emitted.
        if is_capslock(key) {
            let intent = match value {
                1 => self.detector.feed(CapsEvent::CapsDown(now)),
                0 => self.detector.feed(CapsEvent::CapsUp(now)),
                _ => None, // ignore caps auto-repeat
            };
            if let Some(i) = intent {
                self.handle_caps_intent(i, &mut fx);
            }
            return fx;
        }

        // Any other key DOWN may resolve a pending caps-hold (tap-hold-press).
        if value == 1
            && let Some(CapsIntent::HoldStart) = self.detector.feed(CapsEvent::OtherKeyDown(now))
        {
            self.enter_momentary(&mut fx);
        }

        // Modifier keys: track state; Super is swallowed, others pass through.
        if let Some(m) = modifier_of(key) {
            if value != 2 {
                self.mods.set(m, value == 1);
            }
            if m != Modifier::Super {
                fx.push(Effect::Emit { code, value });
            }
            return fx;
        }

        let is_press = value == 1;
        let is_repeat = value == 2;
        let mode = self.mode();
        let chord = Chord {
            mods: self.mods,
            code,
        };

        log::trace!(
            "key {:?} val={value} mode={mode} mods={:?}",
            KeyCode(code),
            self.mods
        );

        // Bound in the current mode?
        if let Some(res) = self.bindings.get(&mode).and_then(|m| m.get(&chord)) {
            let res = res.clone();
            if is_press || (is_repeat && res.repeat) {
                log::debug!(
                    "bound {:?} mode={mode} → {:?} (exit_after={}, repeat={})",
                    KeyCode(code),
                    res.action,
                    res.exit_after,
                    res.repeat
                );
                self.run_binding(&res, is_repeat, &mut fx);
            }
            return fx; // a bound key is always swallowed
        }

        // Paradigm remap (macOS-Command: Super+letter → Ctrl+letter), via the
        // praxis RemapSet. Only when Super is the SOLE modifier — the remap source
        // is `Super + key`, so Super+Shift+C is a *different* combo and must not
        // fire the Super+C remap (which, with Shift held, would inject Ctrl+Shift+C).
        if self.mods
            == (Mods {
                sup: true,
                shift: false,
                ctrl: false,
                alt: false,
            })
            && let Some(key) = keycode_to_key(code)
        {
            let from = KeyCombo::new(key).with_mod(PxMod::Super);
            if let Some(to) = self.remaps.apply(&from).cloned() {
                if is_press {
                    // In a terminal a bare Ctrl+C (VINTR under ISIG) sends SIGINT
                    // to the foreground process group (POSIX termios) — the macOS
                    // copy gesture would KILL the running job. So re-target
                    // copy/paste to the universal terminal combo Ctrl+Shift+C/V,
                    // and suppress every other remap so we never inject readline
                    // control codes (Ctrl+A start-of-line, Ctrl+W kill-word, …).
                    // Real Ctrl+C (no Super) still passes straight through as SIGINT.
                    let target = if self.active_is_terminal() {
                        terminal_copy_paste_target(&to)
                    } else {
                        Some(to.clone())
                    };
                    match target.as_ref().and_then(keycombo_to_chord) {
                        Some(chord) => {
                            log::debug!("remap super+{:?} → {}", KeyCode(code), to.display());
                            self.emit_chord_tap(&chord, &mut fx);
                        }
                        None => {
                            log::debug!("remap suppressed/unmappable: super+{:?}", KeyCode(code))
                        }
                    }
                }
                return fx; // swallow the original super-combo (incl. release/repeat)
            }
        }

        // A Super-combo that matched neither a binding nor a remap is still OWNED
        // by vogix — Super was already swallowed above, so falling through to the
        // passthrough re-emit would type the bare base key (e.g. Super+h → a stray
        // "h") into the focused app. Drop any unmapped Super-combo instead.
        if self.mods.sup {
            if is_press {
                log::debug!("unmapped super-combo swallowed: super+{:?}", KeyCode(code));
            }
            return fx;
        }

        // Unbound: catchall swallows; passthrough re-emits.
        if *self.catchall.get(&mode).unwrap_or(&false) {
            if is_press {
                log::debug!(
                    "unbound {:?} mode={mode} — swallowed (catchall)",
                    KeyCode(code)
                );
            }
        } else {
            log::trace!(
                "unbound {:?} mode={mode} — re-emitted (passthrough)",
                KeyCode(code)
            );
            fx.push(Effect::Emit { code, value });
        }
        fx
    }

    fn run_binding(&mut self, res: &Resolved, is_repeat: bool, fx: &mut Vec<Effect>) {
        match &res.action {
            // Mode switches don't repeat.
            ActionKind::Submap(target) if !is_repeat => {
                let action = if target == "reset" {
                    ModeTransition::ExitToRoot
                } else {
                    ModeTransition::Switch(ModeId::new(target))
                };
                self.apply_and_note(action, fx);
            }
            ActionKind::Submap(_) => {}
            ActionKind::Dispatch(s) => {
                fx.push(Effect::Dispatch(s.clone()));
                if res.exit_after && !is_repeat {
                    self.apply_and_note(ModeTransition::ExitToRoot, fx);
                }
            }
        }
    }

    fn handle_caps_intent(&mut self, intent: CapsIntent, fx: &mut Vec<Effect>) {
        match intent {
            CapsIntent::HoldStart => self.enter_momentary(fx),
            CapsIntent::HoldEnd => self.apply_and_note(ModeTransition::ReleaseHold, fx),
            CapsIntent::Tap => {
                if self.at_root() {
                    if let Some(t) = self.caps_target.clone() {
                        self.apply_and_note(ModeTransition::EnterSticky(t), fx);
                    }
                } else {
                    self.apply_and_note(ModeTransition::ExitToRoot, fx);
                }
            }
        }
    }

    fn enter_momentary(&mut self, fx: &mut Vec<Effect>) {
        // Only enter from root; if already in a (sticky) mode, keep it.
        if self.at_root()
            && let Some(t) = self.caps_target.clone()
        {
            self.apply_and_note(ModeTransition::EnterMomentary(t), fx);
        }
    }

    /// Emit a full press+release of a chord (modifiers wrap the base key).
    fn emit_chord_tap(&self, c: &Chord, fx: &mut Vec<Effect>) {
        let mut mod_codes = Vec::new();
        if c.mods.ctrl {
            mod_codes.push(modifier_code(Modifier::Ctrl).0);
        }
        if c.mods.shift {
            mod_codes.push(modifier_code(Modifier::Shift).0);
        }
        if c.mods.alt {
            mod_codes.push(modifier_code(Modifier::Alt).0);
        }
        if c.mods.sup {
            mod_codes.push(modifier_code(Modifier::Super).0);
        }
        for m in &mod_codes {
            fx.push(Effect::Emit { code: *m, value: 1 });
        }
        fx.push(Effect::Emit {
            code: c.code,
            value: 1,
        });
        fx.push(Effect::Emit {
            code: c.code,
            value: 0,
        });
        for m in mod_codes.iter().rev() {
            fx.push(Effect::Emit { code: *m, value: 0 });
        }
    }

    /// Initialize tracked modifier state from the keys physically held at grab
    /// time. The router starts with no modifiers held, so a Shift/Ctrl/Alt still
    /// down across an engine (re)start would be lost — chords would mismatch
    /// until the user re-pressed it. Only modifiers are synced; normal keys are
    /// matched on their own subsequent down events.
    pub fn sync_held_keys(&mut self, held: impl IntoIterator<Item = u16>) {
        for code in held {
            if let Some(m) = modifier_of(KeyCode(code)) {
                self.mods.set(m, true);
            }
        }
    }

    /// Key-up effects releasing every modifier the router currently believes is
    /// held — for a clean shutdown. The engine re-emits Shift/Ctrl/Alt as the
    /// user holds them; if it exits (SIGTERM on `systemctl stop`/restart) while
    /// one is down, the compositor is left with a stuck modifier that corrupts
    /// every later keystroke (a lockout-class bug). Super is never emitted (it is
    /// swallowed), so it needs no release.
    pub fn release_held_modifiers(&self) -> Vec<Effect> {
        [
            (self.mods.shift, Modifier::Shift),
            (self.mods.ctrl, Modifier::Ctrl),
            (self.mods.alt, Modifier::Alt),
        ]
        .into_iter()
        .filter(|(held, _)| *held)
        .map(|(_, m)| Effect::Emit {
            code: modifier_code(m).0,
            value: 0,
        })
        .collect()
    }
}

/// If a remap target is copy or paste (Ctrl+C / Ctrl+V), return its
/// terminal-safe equivalent Ctrl+Shift+C / Ctrl+Shift+V (the universal Linux
/// terminal copy/paste). Returns `None` for any other remap, which the caller
/// then suppresses in a terminal. Works on the praxis `KeyCombo`, so the
/// copy/paste identity is the paradigm's `Key::Letter`, not a hardwired keycode.
fn terminal_copy_paste_target(to: &KeyCombo) -> Option<KeyCombo> {
    let is_copy_paste =
        matches!(to.key, Key::Letter('c') | Key::Letter('v')) && to.modifiers == [PxMod::Ctrl];
    is_copy_paste.then(|| {
        KeyCombo::new(to.key.clone())
            .with_mod(PxMod::Ctrl)
            .with_mod(PxMod::Shift)
    })
}

/// Convert a praxis `KeyCombo` to the evdev `Chord` the I/O shell emits. `None`
/// if the key has no evdev keycode (e.g. a mouse button).
fn keycombo_to_chord(combo: &KeyCombo) -> Option<Chord> {
    Some(Chord {
        mods: modifiers_to_mods(&combo.modifiers),
        code: key_to_keycode(&combo.key)?,
    })
}

/// Drain pending bytes from the Hyprland event stream and update the router's
/// active-window class from any `activewindow>>class,title` lines. Returns
/// `true` if the stream closed/errored (the caller drops it and reconnects).
/// The stream is non-blocking; `WouldBlock` just means we've drained it.
fn read_window_events(
    stream: &mut std::os::unix::net::UnixStream,
    buf: &mut String,
    router: &mut Router,
) -> bool {
    use std::io::Read;
    let mut chunk = [0u8; 4096];
    loop {
        match stream.read(&mut chunk) {
            Ok(0) => return true, // EOF — the compositor closed the stream.
            Ok(n) => {
                buf.push_str(&String::from_utf8_lossy(&chunk[..n]));
                while let Some(nl) = buf.find('\n') {
                    let line: String = buf.drain(..=nl).collect();
                    if line.starts_with("activewindow>>") {
                        // Some(class) → that window; None (empty class / focus
                        // lost) → fail-safe to non-terminal.
                        router.set_active_class(super::hypr::parse_activewindow_event(&line));
                    }
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => return false,
            Err(_) => return true, // real error → reconnect
        }
    }
}

// ── I/O shell ────────────────────────────────────────────────────────────────

/// Path of the input engine's single-instance lock
/// (`$XDG_RUNTIME_DIR/vogix-input.lock`, falling back to `/tmp`).
fn single_instance_lock_path() -> std::path::PathBuf {
    let dir = std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".to_string());
    std::path::Path::new(&dir).join("vogix-input.lock")
}

/// Take a non-blocking exclusive advisory lock on `path` (creating it). Returns
/// the held [`std::fs::File`] (keep it alive — the lock releases when it drops),
/// or an error if another process already holds it.
fn lock_exclusive(path: &std::path::Path) -> crate::errors::Result<std::fs::File> {
    use crate::errors::VogixError;
    use std::os::fd::AsRawFd;
    let file = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(false)
        .open(path)
        .map_err(|e| VogixError::Config(format!("open lock {}: {e}", path.display())))?;
    // LOCK_EX | LOCK_NB: take it, or fail immediately if another fd holds it.
    if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
        let err = std::io::Error::last_os_error();
        return Err(VogixError::Config(format!(
            "another vogix-input already holds {} ({err}); refusing to grab the keyboard \
             (a second engine would collide and drop keystrokes)",
            path.display()
        )));
    }
    Ok(file)
}

/// Acquire the engine's single-instance lock (see [`run`]).
fn acquire_single_instance_lock() -> crate::errors::Result<std::fs::File> {
    lock_exclusive(&single_instance_lock_path())
}

/// Open the uinput emit device, grab the keyboard, and pump events through a
/// [`Router`] built from the loaded schema. Blocks until interrupted.
///
/// **Order matters.** uinput is opened BEFORE the keyboard grab so that an
/// uinput failure (e.g. EACCES because the user isn't in the `uinput` group
/// yet — a stale-session gotcha after a fresh nix rebuild) returns without
/// ever taking the keyboard. The previous order let the grab succeed first,
/// then failed on uinput, then exited; with `Restart=on-failure` that loop
/// briefly stole the keyboard every cycle, breaking login screens and TTY
/// input. Real keystrokes never reach the rest of the system during a grab,
/// so we'd rather refuse to start than half-start.
///
/// VM-tested only until proven on the host (it takes over the keyboard).
pub fn run(schema: Schema) -> crate::errors::Result<()> {
    use crate::errors::VogixError;
    use evdev::{AttributeSet, EventType, InputEvent};
    use std::os::fd::AsRawFd;
    use std::time::Instant;

    // Single-instance guard FIRST, before any grab. Two engines grabbing the
    // same keyboards at once collide and drop keystrokes (observed when a
    // restart overlapped the previous instance). A second instance fails to
    // take this lock and exits cleanly WITHOUT racing the grab. The lock
    // releases when this process exits (fd close), so a clean restart hands off
    // as soon as the old instance is gone. `_instance_lock` must stay bound for
    // the whole run loop to keep the lock held.
    let _instance_lock = acquire_single_instance_lock()?;

    // vogix owns the keyboard regardless of whether a compositor is running.
    // It has two output paths and only ONE of them depends on a compositor:
    //   1. normal keys are re-emitted on the virtual uinput device — read by
    //      the TTY console AND by any Wayland compositor, so typing always
    //      works (this is what makes vogix the universal keybinding layer,
    //      independent of Hyprland);
    //   2. WM actions are sent as best-effort control messages to whatever
    //      compositor is up — if none is, they are simply dropped.
    // So we do NOT gate the grab on a compositor. Discovery is lazy and
    // self-healing (see `execute`): a compositor that appears later, or
    // restarts, is picked up on the next dispatch.
    let mut hypr = super::hypr::Hypr::discover();
    match &hypr {
        Some(h) => log::info!("compositor socket: {}", h.socket_path().display()),
        None => log::warn!(
            "no compositor control socket yet; WM actions are best-effort until one \
             appears (typing still works via re-emit)"
        ),
    }

    // Build the virtual uinput device FIRST. If this fails (most commonly
    // because we don't have rw on /dev/uinput), we exit before grabbing any
    // real keyboard — the user can keep typing into their login screen / TTY
    // while we figure out the permission problem.
    let mut keys = AttributeSet::<KeyCode>::new();
    for c in 1u16..=0x2ff {
        keys.insert(KeyCode(c));
    }
    let mut vdev = evdev::uinput::VirtualDevice::builder()
        .map_err(|e| VogixError::Config(format!("uinput: {e}")))?
        .name(VIRTUAL_NAME)
        .with_keys(&keys)
        .map_err(|e| VogixError::Config(format!("uinput keys: {e}")))?
        .build()
        .map_err(|e| VogixError::Config(format!("uinput build: {e}")))?;

    // Enumerate keyboard-like devices (have the 'A' key) — EXCLUDING our own
    // virtual device. Because uinput is built before this enumerate (so a uinput
    // failure never leaves a real keyboard grabbed), `vogix-input` is already
    // present and itself advertises KEY_A; grabbing it would make the engine
    // capture the very events it re-emits (a feedback loop that drops all
    // passthrough — caught by the VM test's "plain key re-emitted" assertion).
    //
    // LIMITATION (tracked): enumeration happens ONCE at startup. A keyboard
    // hot-plugged after the engine starts is not grabbed, so its keys reach the
    // compositor unremapped (no Super-swallow, no modes) until the service is
    // restarted — a plausible "keybindings randomly don't work" cause. The
    // proper fix is a udev/inotify monitor that grabs new keyboards as they
    // appear; deferred.
    // Pick which devices to grab. Two stages, lockout-safe:
    //   BROAD  — every non-self device advertising a normal key (the universe).
    //   NARROW — keep only real text keyboards, dropping the YubiKey/audio HID/etc.
    //            via `DeviceFilter`. The strict filter may only narrow a NON-empty
    //            set; if it would leave nothing (a misconfigured filter), keep the
    //            broad set instead — excluding the user's only keyboard is a total
    //            lockout (`selection`'s fail-safe).
    let filter = schema.device_filter();
    let broad: Vec<(std::path::PathBuf, evdev::Device)> = evdev::enumerate()
        .filter(|(_, d)| {
            d.name() != Some(VIRTUAL_NAME)
                && d.supported_keys()
                    .is_some_and(|k| k.contains(KeyCode::KEY_A))
        })
        .collect();
    let flags: Vec<bool> = broad
        .iter()
        .map(|(_, d)| filter.is_text_keyboard(d))
        .collect();
    let (keep_idx, widened) = super::devfilter::selection(&flags);
    if widened {
        log::warn!(
            "device filter matched no keyboards among {} candidate(s); grabbing all to \
             avoid lockout — check deviceFilter excludeVendors/excludeNameSubstrings",
            broad.len()
        );
    }
    let keep: HashSet<usize> = keep_idx.into_iter().collect();
    // Stable slots: each grabbed keyboard keeps its index for the life of the
    // engine; a slot is `None`d on unplug and reused/extended on hotplug.
    let mut slots: Vec<Option<Grabbed>> = Vec::new();
    for (i, (node, mut d)) in broad.into_iter().enumerate() {
        // Device identity (name/vendor) is hardware metadata, not keystrokes —
        // logging it closes the "is this the right device?" diagnosis gap (the
        // exact confusion of the flaky-keyboard incident) without keylogging.
        let id = d.input_id();
        let (vendor, product) = (id.vendor(), id.product());
        let name = d.name().unwrap_or("?").to_string();
        if keep.contains(&i) {
            // Grab here (was a separate pass): we own the keyboard from now on.
            d.grab()
                .map_err(|e| VogixError::Config(format!("cannot grab keyboard {name:?}: {e}")))?;
            log::info!("grabbing keyboard {name:?} ({vendor:04x}:{product:04x})");
            slots.push(Some(Grabbed {
                dev: d,
                node,
                meta: health::DeviceMeta {
                    name,
                    vendor,
                    product,
                },
                counters: health::Counters::default(),
            }));
        } else {
            log::info!(
                "not grabbing {name:?} ({vendor:04x}:{product:04x}) — not a text keyboard / excluded"
            );
        }
    }
    if slots.is_empty() {
        return Err(VogixError::Config("no keyboard devices found".into()));
    }

    let mut router = Router::new(&schema);

    // Resync modifier state: a Shift/Ctrl/Alt still physically held across an
    // engine (re)start would otherwise be lost (the router starts empty),
    // mismatching chords until the user re-presses it.
    for g in slots.iter().flatten() {
        if let Ok(state) = g.dev.get_key_state() {
            router.sync_held_keys(state.iter().map(|k| k.0));
        }
    }

    // Active-window tracking for the context-aware Super→Ctrl remap. The engine
    // grabs evdev, which is blind to the focused window, so the window class
    // (needed to know we're in a terminal) must come from the compositor:
    // subscribe to Hyprland's event socket and SEED the current class. Best
    // effort — with no compositor the remap fails safe (every window treated as
    // non-terminal → plain Ctrl+C, the prior behaviour).
    let mut win_events: Option<std::os::unix::net::UnixStream> = None;
    let mut win_buf = String::new();
    if let Some(h) = hypr.as_ref() {
        router.set_active_class(h.query_active_class());
        win_events = h.connect_events().ok();
    }

    // Paint the current mode's border once at startup, so the mode-visibility cue
    // is correct from the start — a prior session/crash may have left the border
    // at a non-app colour across this engine restart.
    execute(&mut vdev, &mut hypr, router.paint_current_mode());

    let start = Instant::now();
    let ms = move |start: &Instant| start.elapsed().as_millis() as u64;

    log::info!(
        "vogix input running: grabbed {} keyboard(s), mode = {}",
        slots.iter().flatten().count(),
        router.mode()
    );

    // Observability: per-device flow counters live IN each `Grabbed` slot (so
    // they can never desync from the device set under hotplug); stuck-key
    // tracking + the health snapshot for `vogix input doctor`. Counts + device
    // identity only — never key identity (see [`super::health`]).
    let pid = std::process::id();
    let mut held = health::HeldKeys::default();
    let mut last_snapshot_ms: u64 = 0;
    health::write_snapshot(&build_snapshot(&slots, &held, &router.mode(), 0, pid));

    // Hotplug: watch /dev/input so a keyboard reconnected AFTER startup is grabbed
    // without a service restart (the old once-at-enumerate limitation). inotify is
    // pure-Rust (no libudev); best-effort — a failure just disables hotplug.
    let mut inotify_buf = [0u8; 4096];
    let mut inotify = inotify::Inotify::init().ok();
    if let Some(ino) = inotify.as_mut() {
        if ino
            .watches()
            .add(
                "/dev/input",
                inotify::WatchMask::CREATE | inotify::WatchMask::ATTRIB,
            )
            .is_err()
        {
            log::warn!("vogix input: inotify watch on /dev/input failed — hotplug disabled");
            inotify = None;
        }
    } else {
        log::warn!(
            "vogix input: inotify init failed — hotplug disabled (reconnect needs a restart)"
        );
    }

    // Clean-shutdown self-pipe. On SIGTERM/SIGINT (`systemctl --user stop` or a
    // restart) we must release any modifier we re-emitted, or the compositor is
    // left with a stuck Ctrl/Shift/Alt that corrupts every later keystroke (a
    // lockout-class bug). ctrlc runs its handler on a dedicated thread; it writes
    // one byte to this pipe so the poll loop below wakes and shuts down cleanly.
    // Best-effort: if the pipe or handler can't be installed we skip graceful
    // release rather than fail to start.
    let mut shutdown_pipe = [0 as libc::c_int; 2];
    let have_shutdown_pipe = unsafe { libc::pipe(shutdown_pipe.as_mut_ptr()) } == 0;
    let (shutdown_r, shutdown_w) = (shutdown_pipe[0], shutdown_pipe[1]);
    if have_shutdown_pipe {
        let _ = ctrlc::set_handler(move || {
            let byte = [1u8];
            unsafe {
                libc::write(shutdown_w, byte.as_ptr() as *const libc::c_void, 1);
            }
        });
    }

    loop {
        // Rebuild the poll set each iteration from the LIVE device slots plus the
        // fixed trailing aux fds. The device count changes under hotplug, so the
        // aux indices can't be constants — derive them from the live count, and
        // map each device poll entry back to its STABLE slot via `fd_slot`. This
        // also eliminates the old fixed-index / `fd=-1`-tombstone bug class.
        let mut pollfds: Vec<libc::pollfd> = Vec::with_capacity(slots.len() + 3);
        let mut fd_slot: Vec<usize> = Vec::new();
        for (i, slot) in slots.iter().enumerate() {
            if let Some(g) = slot {
                pollfds.push(libc::pollfd {
                    fd: g.dev.as_raw_fd(),
                    events: libc::POLLIN,
                    revents: 0,
                });
                fd_slot.push(i);
            }
        }
        let aux_base = pollfds.len();
        let inotify_idx = aux_base;
        let shutdown_idx = aux_base + 1;
        let event_idx = aux_base + 2;
        pollfds.push(libc::pollfd {
            fd: inotify.as_ref().map_or(-1, |ino| ino.as_raw_fd()),
            events: libc::POLLIN,
            revents: 0,
        });
        pollfds.push(libc::pollfd {
            fd: if have_shutdown_pipe { shutdown_r } else { -1 },
            events: libc::POLLIN,
            revents: 0,
        });
        pollfds.push(libc::pollfd {
            fd: win_events.as_ref().map_or(-1, |s| s.as_raw_fd()),
            events: libc::POLLIN,
            revents: 0,
        });

        // Wake at the EARLIEST of: the caps tap-hold deadline, the sticky-idle
        // auto-revert deadline, and a 2s retry while the active-window stream is
        // down. -1 = block indefinitely when nothing is pending.
        let now_ms = ms(&start);
        let mut timeout: i32 = -1;
        for dl in [router.deadline(), router.idle_deadline()]
            .into_iter()
            .flatten()
        {
            let t = dl.saturating_sub(now_ms) as i32;
            timeout = if timeout < 0 { t } else { timeout.min(t) };
        }
        if win_events.is_none() {
            timeout = if timeout < 0 { 2000 } else { timeout.min(2000) };
        }
        let n = unsafe { libc::poll(pollfds.as_mut_ptr(), pollfds.len() as libc::nfds_t, timeout) };
        if n < 0 {
            let err = std::io::Error::last_os_error();
            if err.kind() == std::io::ErrorKind::Interrupted {
                continue;
            }
            return Err(VogixError::Config(format!("poll: {err}")));
        }
        if n == 0 {
            // Lazily (re)connect the active-window stream when the compositor
            // appeared or restarted, re-seeding the class. After a compositor
            // RESTART the cached `hypr` still points at the DEAD instance (it is
            // not None), so we must re-discover whenever the stream is down — not
            // only when hypr is None — or connect_events would hit ECONNREFUSED on
            // the dead .socket2.sock forever and terminal detection would freeze.
            if win_events.is_none() {
                if hypr.is_none() {
                    hypr = super::hypr::Hypr::discover();
                }
                let connected = hypr
                    .as_ref()
                    .and_then(|h| h.connect_events().ok().map(|s| (h.query_active_class(), s)));
                match connected {
                    Some((class, s)) => {
                        router.set_active_class(class);
                        win_events = Some(s);
                        log::debug!("vogix input: active-window stream (re)connected");
                    }
                    // Connect failed: drop the (possibly stale) handle so the next
                    // pass re-discovers the live instance after a compositor restart.
                    None => hypr = None,
                }
            }
            let now = ms(&start);
            execute(&mut vdev, &mut hypr, router.on_timeout(now));
            execute(&mut vdev, &mut hypr, router.on_idle(now));
            health::write_snapshot(&build_snapshot(&slots, &held, &router.mode(), now, pid));
            last_snapshot_ms = now;
            continue;
        }
        // Shutdown signalled → release any modifier we hold, then exit cleanly.
        if have_shutdown_pipe && pollfds[shutdown_idx].revents & libc::POLLIN != 0 {
            log::info!("vogix input: shutdown signal received — releasing held modifiers");
            execute(&mut vdev, &mut hypr, router.release_held_modifiers());
            // Let the kernel deliver the key-up events before the uinput fd closes
            // on return (device teardown isn't ordered after the compositor reads
            // them) — otherwise a modifier could be left stuck across the restart.
            std::thread::sleep(std::time::Duration::from_millis(20));
            return Ok(());
        }
        // Active-window class updates from Hyprland's event stream (drives the
        // terminal-aware Super→Ctrl remap). On close/error → drop + reconnect.
        if win_events.is_some()
            && pollfds[event_idx].revents & (libc::POLLIN | libc::POLLHUP | libc::POLLERR) != 0
            && read_window_events(win_events.as_mut().unwrap(), &mut win_buf, &mut router)
        {
            win_events = None;
        }
        // Hotplug: a new /dev/input node appeared → grab it if it's a text
        // keyboard (reusing the startup predicate), so a reconnected keyboard
        // works without a service restart.
        if pollfds[inotify_idx].revents & libc::POLLIN != 0
            && let Some(ino) = inotify.as_mut()
        {
            handle_hotplug(ino, &mut inotify_buf, &filter, &mut slots, &mut router);
        }
        // Device fds: map each ready poll entry back to its STABLE slot.
        for j in 0..aux_base {
            let slot = fd_slot[j];
            let revents = pollfds[j].revents;
            // An unplugged grabbed keyboard raises POLLHUP/POLLERR (never POLLIN);
            // left in, poll() would report it ready forever and spin at 100% CPU.
            // POLLHUP is the AUTHORITATIVE drop signal (reconciled by live fd, not
            // by a reused devnode): None the slot — the `Grabbed`'s `Device`
            // ungrabs on Drop — and a future hotplug re-grabs it.
            if revents & (libc::POLLHUP | libc::POLLERR | libc::POLLNVAL) != 0 {
                if let Some(g) = slots[slot].take() {
                    log::info!(
                        "vogix input: keyboard {:?} disconnected — released",
                        g.meta.name
                    );
                }
                continue;
            }
            if revents & libc::POLLIN == 0 {
                continue;
            }
            let Some(g) = slots[slot].as_mut() else {
                continue;
            };
            let events: Vec<(u16, i32)> = match g.dev.fetch_events() {
                Ok(it) => it
                    .filter(|e| e.event_type() == EventType::KEY)
                    .map(|e| (e.code(), e.value()))
                    .collect(),
                Err(_) => continue,
            };
            for (code, value) in events {
                let now = ms(&start);
                held.on_event(code, value, now);
                let fx = router.on_key(code, value, now);
                g.counters.record(&fx, now);
                execute(&mut vdev, &mut hypr, fx);
            }
        }
        // Refresh the health snapshot on a coarse wall-clock interval even under
        // steady typing (the idle branch never fires while keys flow), so `doctor`
        // always sees a fresh per-device view — including the "went silent" tell.
        let now = ms(&start);
        if now.saturating_sub(last_snapshot_ms) >= 1000 {
            last_snapshot_ms = now;
            health::write_snapshot(&build_snapshot(&slots, &held, &router.mode(), now, pid));
        }
        let _ = InputEvent::new; // keep import meaningful across cfgs
    }
}

/// Open + grab a hotplugged `/dev/input` node as a keyboard slot, or `None` if
/// it isn't a real text keyboard (the SAME `DeviceFilter` predicate as startup)
/// or can't be opened/grabbed. Best-effort: a reject leaves it ungrabbed (its
/// keys reach the compositor unremapped — degraded, never a lockout), retried on
/// the next inotify event.
fn try_grab_keyboard(
    node: &std::path::Path,
    filter: &super::devfilter::DeviceFilter,
) -> Option<Grabbed> {
    let mut dev = evdev::Device::open(node).ok()?;
    if dev.name() == Some(VIRTUAL_NAME) || !filter.is_text_keyboard(&dev) {
        return None;
    }
    let id = dev.input_id();
    let (vendor, product) = (id.vendor(), id.product());
    let name = dev.name().unwrap_or("?").to_string();
    if let Err(e) = dev.grab() {
        log::warn!("vogix input: cannot grab hotplugged {name:?}: {e}");
        return None;
    }
    log::info!("vogix input: grabbed hotplugged keyboard {name:?} ({vendor:04x}:{product:04x})");
    Some(Grabbed {
        dev,
        node: node.to_path_buf(),
        meta: health::DeviceMeta {
            name,
            vendor,
            product,
        },
        counters: health::Counters::default(),
    })
}

/// Drain inotify events and grab any newly-appeared text keyboard. Dedups by
/// devnode (inotify fires CREATE then ATTRIB for one node) and reuses a freed
/// (`None`) slot before pushing, so repeated unplug/replug keeps `slots` bounded.
fn handle_hotplug(
    inotify: &mut inotify::Inotify,
    buf: &mut [u8],
    filter: &super::devfilter::DeviceFilter,
    slots: &mut Vec<Option<Grabbed>>,
    router: &mut Router,
) {
    let Ok(events) = inotify.read_events(buf) else {
        return;
    };
    for event in events {
        let Some(name) = event.name else {
            continue;
        };
        // Only /dev/input/event* nodes are evdev devices.
        if !name.to_string_lossy().starts_with("event") {
            continue;
        }
        let node = std::path::Path::new("/dev/input").join(name);
        if slots.iter().flatten().any(|g| g.node == node) {
            continue; // already grabbed
        }
        if let Some(g) = try_grab_keyboard(&node, filter) {
            // A modifier held across the reconnect would otherwise be lost.
            if let Ok(state) = g.dev.get_key_state() {
                router.sync_held_keys(state.iter().map(|k| k.0));
            }
            match slots.iter_mut().find(|s| s.is_none()) {
                Some(free) => *free = Some(g),
                None => slots.push(Some(g)),
            }
        }
    }
}

/// Assemble a health snapshot from the live slots' counters + device identity.
/// Carries NO key identity — `silent_ms` (time since a device's last event) is
/// the "this device went quiet" tell that localises a flaky keyboard to hardware.
fn build_snapshot(
    slots: &[Option<Grabbed>],
    held: &health::HeldKeys,
    mode: &str,
    now_ms: u64,
    pid: u32,
) -> health::HealthSnapshot {
    let devices = slots
        .iter()
        .flatten()
        .map(|g| {
            let silent_ms = if g.counters.last_event_ms == 0 {
                now_ms
            } else {
                now_ms.saturating_sub(g.counters.last_event_ms)
            };
            health::DeviceHealth {
                name: g.meta.name.clone(),
                vendor: g.meta.vendor,
                product: g.meta.product,
                counters: g.counters.clone(),
                silent_ms,
            }
        })
        .collect();
    let (stuck_count, stuck_oldest_ms) = held.stuck(now_ms, health::STUCK_MS);
    health::HealthSnapshot {
        pid,
        uptime_ms: now_ms,
        mode: mode.to_string(),
        devices,
        stuck_count,
        stuck_oldest_ms,
    }
}

fn execute(
    vdev: &mut evdev::uinput::VirtualDevice,
    hypr: &mut Option<super::hypr::Hypr>,
    fx: Vec<Effect>,
) {
    use evdev::{EventType, InputEvent};
    for e in fx {
        match e {
            Effect::Emit { code, value } => {
                // A wedged virtual device would otherwise silently eat all
                // typing — surface it instead of dropping the error.
                if let Err(e) = vdev.emit(&[InputEvent::new(EventType::KEY.0, code, value)]) {
                    log::warn!("uinput emit failed (code={code} value={value}): {e}");
                }
            }
            Effect::Dispatch(action) => {
                // Best-effort + self-healing: if we have no socket yet (no
                // compositor when we started, or it restarted) try to find one
                // now. A dispatch error means the socket went stale, so drop it
                // and re-discover on the next action. Either way a missing
                // compositor never blocks the engine — the WM action is just
                // dropped while typing (re-emit) keeps working.
                if hypr.is_none() {
                    *hypr = super::hypr::Hypr::discover();
                }
                match hypr.as_ref() {
                    Some(h) => match h.dispatch(&action) {
                        Ok(()) => log::debug!("dispatch ok: '{action}'"),
                        Err(e) => {
                            log::warn!(
                                "dispatch '{action}' failed: {e}; will re-discover compositor"
                            );
                            *hypr = None;
                        }
                    },
                    None => log::warn!("dispatch '{action}' dropped: no compositor socket"),
                }
            }
            Effect::Keyword { key, value } => {
                // Mode-visibility surface (border colour). Same best-effort,
                // self-healing path as Dispatch — a missing compositor just means
                // the border isn't repainted; it never blocks the engine.
                if hypr.is_none() {
                    *hypr = super::hypr::Hypr::discover();
                }
                match hypr.as_ref() {
                    Some(h) => match h.set_keyword(&key, &value) {
                        Ok(()) => log::debug!("keyword ok: '{key} {value}'"),
                        Err(e) => {
                            log::warn!("keyword '{key} {value}' failed: {e}; will re-discover");
                            *hypr = None;
                        }
                    },
                    None => log::warn!("keyword '{key} {value}' dropped: no compositor socket"),
                }
            }
            Effect::ModeChanged(mode) => {
                // waybar surface: publish the active mode to a state file a custom
                // waybar module reads (the module wiring is a follow-on).
                publish_mode(&mode);
                log::info!("mode → {mode}");
            }
        }
    }
}

/// Publish the active mode to `~/.local/state/vogix/current-mode` for the waybar
/// mode surface. Best-effort: a write failure never affects input handling.
fn publish_mode(mode: &str) {
    let path = crate::config::Config::state_dir().join("current-mode");
    let _ = std::fs::write(path, mode);
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    const SCHEMA: &str = r#"{
      "modeGraph": { "root": "app", "modes": {
        "app":     { "parent": null,  "type": "normal" },
        "desktop": { "parent": "app", "type": "submap" },
        "move":    { "parent": "app", "type": "submap" },
        "resize":  { "parent": "app", "type": "submap" },
        "console": { "parent": "app", "type": "passthrough" }
      }},
      "keybindings": { "modKey": "super", "layers": {
        "desktopToggle": { "hold": "capslock", "tapHoldMs": 250, "holdAction": "f23" }
      }},
      "terminalClasses": ["kitty"],
      "modeColors": {
        "desktop": { "active": "rgb(89b4fa)", "inactive": "rgb(313244)" }
      },
      "modes": {
        "app": { "exit": "escape", "bindings": {
          "ws1": { "key": "super + 1", "action": "workspace, 1" },
          "launcher": { "key": "super + space", "action": "exec, walker" },
          "enterDesktopHold": { "key": "F23", "action": "submap, desktop" }
        }},
        "desktop": { "exit": "escape", "bindings": {
          "focusLeft": { "key": "h", "action": "movefocus, l", "repeat": true },
          "moveShift": { "key": "shift + h", "action": "movewindow, l", "repeat": true },
          "enterMove": { "key": "m", "action": "submap, move" },
          "fullscreen": { "key": "f", "action": "fullscreen" },
          "terminal": { "key": "t", "action": "exec, $TERMINAL", "exitAfter": true },
          "close": { "key": "q", "action": "killactive,", "exitAfter": true }
        }},
        "move": { "exit": "escape", "bindings": {
          "moveLeft": { "key": "h", "action": "movewindow, l", "repeat": true }
        }},
        "resize": { "exit": "escape", "bindings": {} },
        "console": { "bindings": {} }
      }
    }"#;

    fn router() -> Router {
        Router::new(&Schema::from_json(SCHEMA).unwrap())
    }

    const CAPS: u16 = KeyCode::KEY_CAPSLOCK.0;
    const H: u16 = KeyCode::KEY_H.0;
    const Q: u16 = KeyCode::KEY_Q.0;
    const M: u16 = KeyCode::KEY_M.0;
    const A: u16 = KeyCode::KEY_A.0;
    const C: u16 = KeyCode::KEY_C.0;
    const SUPER: u16 = KeyCode::KEY_LEFTMETA.0;
    const CTRL: u16 = KeyCode::KEY_LEFTCTRL.0;
    const ESC: u16 = KeyCode::KEY_ESC.0;
    const SPACE: u16 = KeyCode::KEY_SPACE.0;

    #[test]
    fn caps_hold_plus_h_focuses_then_release_returns_to_app() {
        let mut r = router();
        // caps down (pending), then h down → resolves hold → enter desktop, focus.
        assert!(r.on_key(CAPS, 1, 0).is_empty());
        let fx = r.on_key(H, 1, 10);
        assert!(fx.contains(&Effect::ModeChanged("desktop".into())));
        assert!(fx.contains(&Effect::Dispatch("movefocus, l".into())));
        assert_eq!(r.mode(), "desktop");
        // h up is swallowed (bound key), caps up → back to app.
        let _ = r.on_key(H, 0, 20);
        let fx = r.on_key(CAPS, 0, 30);
        assert!(fx.contains(&Effect::ModeChanged("app".into())));
        assert_eq!(r.mode(), "app");
    }

    #[test]
    fn caps_tap_toggles_sticky_desktop() {
        let mut r = router();
        r.on_key(CAPS, 1, 0);
        let fx = r.on_key(CAPS, 0, 50); // quick click → tap
        assert!(fx.contains(&Effect::ModeChanged("desktop".into())));
        assert_eq!(r.mode(), "desktop");
        // a key now stays in desktop (sticky); release doesn't exit.
        let fx = r.on_key(H, 1, 100);
        assert!(fx.contains(&Effect::Dispatch("movefocus, l".into())));
        assert_eq!(r.mode(), "desktop");
        // click caps again → exit.
        r.on_key(CAPS, 1, 200);
        let fx = r.on_key(CAPS, 0, 240);
        assert!(fx.contains(&Effect::ModeChanged("app".into())));
        assert_eq!(r.mode(), "app");
    }

    #[test]
    fn sticky_mode_auto_reverts_after_idle() {
        let mut r = router();
        r.on_key(CAPS, 1, 0);
        r.on_key(CAPS, 0, 50); // tap → sticky desktop
        assert_eq!(r.mode(), "desktop");
        // Not yet idle → no revert.
        assert!(r.on_idle(1000).is_empty());
        assert_eq!(r.mode(), "desktop");
        // Past the idle threshold → auto-revert to app (forgotten sticky heals).
        let fx = r.on_idle(50 + crate::input::schema::DEFAULT_STICKY_IDLE_MS + 1);
        assert!(fx.contains(&Effect::ModeChanged("app".into())));
        assert_eq!(r.mode(), "app");
    }

    #[test]
    fn momentary_mode_does_not_idle_revert() {
        // Only STICKY modes idle-revert; a held (momentary) mode is user-maintained.
        let mut r = router();
        r.on_key(CAPS, 1, 0);
        r.on_key(H, 1, 10); // caps-hold+h → momentary desktop
        assert_eq!(r.mode(), "desktop");
        assert!(
            r.on_idle(crate::input::schema::DEFAULT_STICKY_IDLE_MS + 100)
                .is_empty(),
            "a momentary mode must not idle-revert"
        );
        assert_eq!(r.mode(), "desktop");
    }

    #[test]
    fn mode_change_paints_the_border_surface() {
        // Entering a mode with a configured colour paints the active-window
        // border (the mode-visibility surface — the cure for mode error).
        let mut r = router();
        r.on_key(CAPS, 1, 0);
        let fx = r.on_key(CAPS, 0, 50); // tap → sticky desktop
        assert!(fx.contains(&Effect::ModeChanged("desktop".into())));
        assert!(
            fx.contains(&Effect::Keyword {
                key: "general:col.active_border".into(),
                value: "rgb(89b4fa)".into()
            }),
            "entering desktop must paint the active border, got {fx:?}"
        );
    }

    #[test]
    fn enter_move_submode_then_exit_after_close() {
        let mut r = router();
        // sticky desktop
        r.on_key(CAPS, 1, 0);
        r.on_key(CAPS, 0, 50);
        // m → switch to move
        let fx = r.on_key(M, 1, 100);
        assert!(fx.contains(&Effect::ModeChanged("move".into())));
        assert_eq!(r.mode(), "move");
        // h in move → movewindow
        let fx = r.on_key(H, 1, 150);
        assert!(fx.contains(&Effect::Dispatch("movewindow, l".into())));
    }

    #[test]
    fn exit_after_returns_to_app() {
        let mut r = router();
        r.on_key(CAPS, 1, 0);
        r.on_key(CAPS, 0, 50); // sticky desktop
        let fx = r.on_key(Q, 1, 100); // q = killactive, exitAfter
        assert!(fx.contains(&Effect::Dispatch("killactive,".into())));
        assert!(fx.contains(&Effect::ModeChanged("app".into())));
        assert_eq!(r.mode(), "app");
    }

    #[test]
    fn repeat_only_refires_repeatable_bindings() {
        let mut r = router();
        r.on_key(CAPS, 1, 0);
        r.on_key(CAPS, 0, 50); // sticky desktop
        // press h, then auto-repeat (value 2) → both dispatch movefocus.
        let p = r.on_key(H, 1, 100);
        let rep = r.on_key(H, 2, 130);
        assert!(p.contains(&Effect::Dispatch("movefocus, l".into())));
        assert!(rep.contains(&Effect::Dispatch("movefocus, l".into())));
    }

    #[test]
    fn super_c_remaps_to_ctrl_c() {
        let mut r = router();
        // hold Super (swallowed), press c → emit Ctrl down, c, c up, Ctrl up.
        assert!(r.on_key(SUPER, 1, 0).is_empty(), "super is swallowed");
        let fx = r.on_key(C, 1, 10);
        assert_eq!(
            fx,
            vec![
                Effect::Emit {
                    code: CTRL,
                    value: 1
                },
                Effect::Emit { code: C, value: 1 },
                Effect::Emit { code: C, value: 0 },
                Effect::Emit {
                    code: CTRL,
                    value: 0
                },
            ]
        );
    }

    #[test]
    fn unmapped_super_combo_is_swallowed_not_typed() {
        // Super+h is neither an app binding nor a remap; vogix OWNS Super, so it
        // must be swallowed — never re-emit a bare 'h' into the focused app.
        let mut r = router();
        assert!(r.on_key(SUPER, 1, 0).is_empty(), "super is swallowed");
        let fx = r.on_key(H, 1, 10);
        assert!(
            fx.is_empty(),
            "an unmapped super-combo must be swallowed, not typed, got {fx:?}"
        );
    }

    #[test]
    fn remap_does_not_fire_with_an_extra_modifier_held() {
        // Super+Shift+C must NOT fire the Super+C→Ctrl+C remap (Shift is physically
        // held, so it would wrongly become Ctrl+Shift+C). With no super+shift+c
        // binding it is swallowed — never a synthesized Ctrl chord.
        let mut r = router();
        let shift = KeyCode::KEY_LEFTSHIFT.0;
        r.on_key(SUPER, 1, 0);
        r.on_key(shift, 1, 5); // shift down (tracked + re-emitted)
        let fx = r.on_key(C, 1, 10);
        assert!(
            !fx.iter()
                .any(|e| matches!(e, Effect::Emit { code, .. } if *code == CTRL)),
            "super+shift+c must not synthesize a Ctrl chord, got {fx:?}"
        );
    }

    #[test]
    fn super_c_in_terminal_is_ctrl_shift_c_not_sigint() {
        // POSIX termios: a bare Ctrl+C in a terminal sends SIGINT. In a terminal
        // the macOS copy gesture must become Ctrl+Shift+C, never bare Ctrl+C.
        let mut r = router();
        r.set_active_class(Some("kitty".into())); // a declared terminal class
        assert!(r.on_key(SUPER, 1, 0).is_empty());
        let shift = KeyCode::KEY_LEFTSHIFT.0;
        assert_eq!(
            r.on_key(C, 1, 10),
            vec![
                Effect::Emit {
                    code: CTRL,
                    value: 1
                },
                Effect::Emit {
                    code: shift,
                    value: 1
                },
                Effect::Emit { code: C, value: 1 },
                Effect::Emit { code: C, value: 0 },
                Effect::Emit {
                    code: shift,
                    value: 0
                },
                Effect::Emit {
                    code: CTRL,
                    value: 0
                },
            ],
            "Super+C in a terminal must be Ctrl+Shift+C, not bare Ctrl+C (SIGINT)"
        );
    }

    #[test]
    fn non_copy_remap_suppressed_in_terminal() {
        // Super+A → Ctrl+A would be readline start-of-line; suppress it entirely
        // in a terminal rather than inject a control code.
        let mut r = router();
        r.set_active_class(Some("kitty".into()));
        r.on_key(SUPER, 1, 0);
        let fx = r.on_key(A, 1, 10);
        assert!(
            fx.is_empty(),
            "a non-copy/paste remap must be suppressed in a terminal, got {fx:?}"
        );
    }

    #[test]
    fn super_c_in_gui_app_is_plain_ctrl_c() {
        // Outside a terminal (and on unknown/None class — fail-safe), the normal
        // macOS-Command remap applies: Super+C → Ctrl+C.
        let mut r = router();
        r.set_active_class(Some("firefox".into())); // not a terminal class
        r.on_key(SUPER, 1, 0);
        assert_eq!(
            r.on_key(C, 1, 10),
            vec![
                Effect::Emit {
                    code: CTRL,
                    value: 1
                },
                Effect::Emit { code: C, value: 1 },
                Effect::Emit { code: C, value: 0 },
                Effect::Emit {
                    code: CTRL,
                    value: 0
                },
            ]
        );
    }

    #[test]
    fn super_number_dispatches_workspace_not_passthrough() {
        let mut r = router();
        r.on_key(SUPER, 1, 0);
        let fx = r.on_key(KeyCode::KEY_1.0, 1, 10);
        assert!(fx.contains(&Effect::Dispatch("workspace, 1".into())));
    }

    #[test]
    fn super_space_dispatches_launcher() {
        // Regression: Super+Space (launcher) stopped working on the live host.
        let mut r = router();
        assert!(r.on_key(SUPER, 1, 0).is_empty(), "super swallowed");
        let fx = r.on_key(KeyCode::KEY_SPACE.0, 1, 10);
        assert!(
            fx.contains(&Effect::Dispatch("exec, walker".into())),
            "super+space must dispatch the launcher, got {fx:?}"
        );
        assert!(
            !fx.contains(&Effect::Emit {
                code: KeyCode::KEY_SPACE.0,
                value: 1
            }),
            "super+space must be swallowed, not typed as a space"
        );
    }

    #[test]
    fn app_mode_passes_through_typing() {
        let mut r = router();
        // plain 'a' in app mode → emitted (typing works).
        let fx = r.on_key(A, 1, 0);
        assert_eq!(fx, vec![Effect::Emit { code: A, value: 1 }]);
    }

    #[test]
    fn app_mode_passes_through_space() {
        // Regression (live host): every letter typed but Space did not. In app
        // (passthrough) mode a bare Space — Super NOT held — must re-emit as a
        // literal space; it must NOT match the `super + space` launcher binding,
        // nor be swallowed. (`super_space_dispatches_launcher` covers Super-held.)
        let mut r = router();
        let down = r.on_key(SPACE, 1, 0);
        assert_eq!(
            down,
            vec![Effect::Emit {
                code: SPACE,
                value: 1
            }],
            "bare Space must type a space in app mode, got {down:?}"
        );
        // release re-emits too (a half-emitted space would stick the key).
        let up = r.on_key(SPACE, 0, 5);
        assert_eq!(
            up,
            vec![Effect::Emit {
                code: SPACE,
                value: 0
            }],
            "bare Space release must re-emit, got {up:?}"
        );
    }

    #[test]
    fn desktop_catchall_swallows_unbound_keys() {
        let mut r = router();
        r.on_key(CAPS, 1, 0);
        r.on_key(CAPS, 0, 50); // sticky desktop
        // 'a' has no desktop binding → swallowed (catchall), not emitted.
        let fx = r.on_key(A, 1, 100);
        assert!(
            fx.is_empty(),
            "catchall must swallow unbound keys, got {fx:?}"
        );
    }

    #[test]
    fn esc_exits_catchall_mode_but_passes_through_in_app() {
        let mut r = router();
        // In app (normal/passthrough) mode Esc must reach the app — re-emitted,
        // never swallowed — even though app declares exit="escape".
        let fx = r.on_key(ESC, 1, 0);
        assert_eq!(
            fx,
            vec![Effect::Emit {
                code: ESC,
                value: 1
            }],
            "Esc must pass through in app (not a catchall mode)"
        );
        assert_eq!(r.mode(), "app");
        // Enter sticky desktop (a catchall mode), then Esc is the safety-net exit.
        r.on_key(CAPS, 1, 10);
        r.on_key(CAPS, 0, 50);
        assert_eq!(r.mode(), "desktop");
        let fx = r.on_key(ESC, 1, 100);
        assert!(
            fx.contains(&Effect::ModeChanged("app".into())),
            "Esc must exit a catchall mode to root, got {fx:?}"
        );
        assert!(
            !fx.contains(&Effect::Emit {
                code: ESC,
                value: 1
            }),
            "Esc is swallowed (consumed as the exit) inside a catchall mode"
        );
        assert_eq!(r.mode(), "app");
    }

    #[test]
    fn caps_tap_just_under_threshold_is_sticky_not_hold() {
        // A lone caps press released just UNDER tapHoldMs (250) is a TAP (enters
        // sticky), never mis-resolved as a hold — the boundary the loaded VM
        // clock can't reliably hit, pinned here with the injected clock.
        let mut r = router();
        r.on_key(CAPS, 1, 0);
        let fx = r.on_key(CAPS, 0, 240); // 240 < 250 → tap
        assert!(
            fx.contains(&Effect::ModeChanged("desktop".into())),
            "240ms lone caps must be a sticky tap, got {fx:?}"
        );
        assert_eq!(r.mode(), "desktop");
    }

    #[test]
    fn caps_hold_at_threshold_is_momentary_not_tap() {
        // A lone caps held to tapHoldMs resolves as a hold on the poll timeout
        // (momentary); releasing returns to app — never left sticky.
        let mut r = router();
        r.on_key(CAPS, 1, 0);
        let fx = r.on_timeout(250); // deadline reached → hold
        assert!(
            fx.contains(&Effect::ModeChanged("desktop".into())),
            "250ms timeout must be a momentary hold, got {fx:?}"
        );
        let fx = r.on_key(CAPS, 0, 300); // release → back to app (NOT sticky)
        assert!(fx.contains(&Effect::ModeChanged("app".into())));
        assert_eq!(r.mode(), "app");
    }

    #[test]
    fn grab_resync_picks_up_a_held_modifier() {
        // A modifier physically held when the engine grabs must be tracked, so a
        // restart mid-hold doesn't lose it (the Mods-starts-empty stuck class).
        let mut r = router();
        r.sync_held_keys([KeyCode::KEY_LEFTSHIFT.0]);
        // And a clean shutdown then releases exactly that modifier — no stuck Shift.
        assert_eq!(
            r.release_held_modifiers(),
            vec![Effect::Emit {
                code: KeyCode::KEY_LEFTSHIFT.0,
                value: 0
            }]
        );
    }

    #[test]
    fn release_held_modifiers_releases_only_what_is_held() {
        let mut r = router();
        // Nothing held → nothing to release.
        assert!(r.release_held_modifiers().is_empty());
        // Ctrl down (tracked + re-emitted), then a clean shutdown releases it.
        r.on_key(CTRL, 1, 0);
        assert_eq!(
            r.release_held_modifiers(),
            vec![Effect::Emit {
                code: CTRL,
                value: 0
            }]
        );
        // Releasing it normally clears the state (no double-release on shutdown).
        r.on_key(CTRL, 0, 10);
        assert!(r.release_held_modifiers().is_empty());
    }

    #[test]
    fn right_hand_modifiers_match_left_swallow_and_reemit() {
        let mut r = router();
        // Right Super is swallowed exactly like left (a user pressing the right
        // Super expects the same ownership), producing no passthrough.
        assert!(
            r.on_key(KeyCode::KEY_RIGHTMETA.0, 1, 0).is_empty(),
            "RIGHTMETA must be swallowed like LEFTMETA"
        );
        // Right Shift/Ctrl/Alt pass through (re-emitted) so typing/app shortcuts work.
        for code in [
            KeyCode::KEY_RIGHTSHIFT.0,
            KeyCode::KEY_RIGHTCTRL.0,
            KeyCode::KEY_RIGHTALT.0,
        ] {
            assert_eq!(
                r.on_key(code, 1, 0),
                vec![Effect::Emit { code, value: 1 }],
                "right-hand non-super modifier must re-emit"
            );
        }
    }

    #[test]
    fn caps_autorepeat_produces_no_intent_and_no_deadline_drift() {
        let mut r = router();
        r.on_key(CAPS, 1, 0); // pending
        let dl = r.deadline();
        // CapsLock auto-repeat (value==2) must be ignored: no effects, deadline unchanged.
        assert!(
            r.on_key(CAPS, 2, 50).is_empty(),
            "caps repeat must produce no effect"
        );
        assert_eq!(
            r.deadline(),
            dl,
            "caps repeat must not extend/reset the deadline"
        );
        assert_eq!(r.mode(), "app", "caps repeat must not change mode");
    }

    proptest! {
        // The Router had no property test. Fuzz arbitrary (code, value, time)
        // streams and assert the integration never panics and never escapes to an
        // undeclared mode (the layer proptests cover the detector/engine in
        // isolation; this covers their composition through on_key).
        #[test]
        fn router_never_panics_and_stays_in_a_valid_mode(
            events in proptest::collection::vec((1u16..600u16, 0i32..3i32, 0u64..3000u64), 0..300)
        ) {
            let mut r = router();
            let valid = ["app", "desktop", "move", "resize", "console"];
            for (code, value, t) in events {
                let _ = r.on_key(code, value, t);
                let _ = r.on_timeout(t);
                prop_assert!(valid.contains(&r.mode().as_str()), "escaped to invalid mode {}", r.mode());
            }
        }
    }

    #[test]
    fn pure_caps_hold_via_timeout_enters_then_release_exits() {
        let mut r = router();
        r.on_key(CAPS, 1, 0); // pending
        let fx = r.on_timeout(250); // deadline → hold resolves
        assert!(fx.contains(&Effect::ModeChanged("desktop".into())));
        assert_eq!(r.mode(), "desktop");
        let fx = r.on_key(CAPS, 0, 400); // release → back to app
        assert!(fx.contains(&Effect::ModeChanged("app".into())));
        assert_eq!(r.mode(), "app");
    }

    // The single-instance guard: a second engine must not take the lock while
    // one already holds it (overlapping grabs collide and drop keystrokes).
    #[test]
    fn single_instance_lock_is_exclusive() {
        let path =
            std::env::temp_dir().join(format!("vogix-input-test-{}.lock", std::process::id()));
        let _ = std::fs::remove_file(&path);

        let held = super::lock_exclusive(&path).expect("first instance takes the lock");
        assert!(
            super::lock_exclusive(&path).is_err(),
            "a second instance must be refused while the first holds the lock"
        );
        drop(held); // first instance exits → lock releases
        let reacquired = super::lock_exclusive(&path).expect("re-acquirable after release");
        drop(reacquired);
        let _ = std::fs::remove_file(&path);
    }
}

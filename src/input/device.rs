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

use super::keys::{Chord, Modifier, Mods, is_capslock, modifier_code, modifier_of, parse_chord};
use super::schema::{ActionKind, Schema, parse_action};
use super::taphold::{CapsDetector, CapsEvent, CapsIntent};
use evdev::KeyCode;
use pr4xis::engine::Engine;
use pr4xis_domains::applied::hmi::input::engine::{ModeTransition, drive, new_input_engine};
use pr4xis_domains::applied::hmi::input::modes::ModeId;
use std::collections::HashMap;

/// A side effect the I/O shell should perform.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Effect {
    /// Emit a key event on the virtual device (passthrough or synthesized).
    Emit { code: u16, value: i32 },
    /// Run a Hyprland action string over the control socket.
    Dispatch(String),
    /// The active mode changed (for visuals / logging).
    ModeChanged(String),
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
    /// Super+code → target chord (e.g. Super+C → Ctrl+C).
    remaps: HashMap<u16, Chord>,
    caps_target: Option<ModeId>,
    root: ModeId,
}

impl Router {
    /// Build a router from the loaded schema.
    pub fn new(schema: &Schema) -> Self {
        let graph = schema.build_mode_graph();
        let root = graph.root.clone();
        let engine = Some(new_input_engine(graph));

        let mut bindings = HashMap::new();
        let mut catchall = HashMap::new();
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
            bindings.insert(mode_name.clone(), map);
        }
        for (name, node) in &schema.mode_graph.modes {
            catchall.insert(name.clone(), node.kind.as_deref() == Some("submap"));
        }

        let mut remaps = HashMap::new();
        for r in schema.super_ctrl_remaps.values() {
            if let (Some(from), Some(to)) = (parse_chord(&r.from), parse_chord(&r.to))
                && from.mods.sup
            {
                remaps.insert(from.code, to);
            }
        }

        Self {
            engine,
            detector: CapsDetector::new(schema.tap_hold_ms()),
            mods: Mods::default(),
            bindings,
            catchall,
            remaps,
            caps_target: schema.caps_target().map(ModeId::new),
            root,
        }
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
            fx.push(Effect::ModeChanged(to));
        }
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

        // Super→Ctrl style remap, when nothing explicit matched.
        if self.mods.sup && self.remaps.contains_key(&code) {
            if is_press {
                let to = self.remaps[&code];
                log::debug!("remap super+{:?} → {:?}", KeyCode(code), KeyCode(to.code));
                self.emit_chord_tap(&to, &mut fx);
            }
            return fx; // swallow the original super-combo (incl. release/repeat)
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

    // The name of our virtual re-emit device. We must NOT grab it back (see the
    // enumerate filter below), or we'd capture our own emitted events.
    const VIRTUAL_NAME: &str = "vogix-input";

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
    let mut devices: Vec<evdev::Device> = evdev::enumerate()
        .map(|(_, d)| d)
        .filter(|d| {
            d.name() != Some(VIRTUAL_NAME)
                && d.supported_keys()
                    .is_some_and(|k| k.contains(KeyCode::KEY_A))
        })
        .collect();
    if devices.is_empty() {
        return Err(VogixError::Config("no keyboard devices found".into()));
    }
    // Grab last. If we got this far, uinput is open and we can route; the
    // grab is the last thing that takes input away from anyone else.
    for d in &mut devices {
        d.grab()
            .map_err(|e| VogixError::Config(format!("cannot grab keyboard: {e}")))?;
    }

    let mut router = Router::new(&schema);
    let start = Instant::now();
    let ms = move |start: &Instant| start.elapsed().as_millis() as u64;

    log::info!(
        "vogix input running: grabbed {} keyboard(s), mode = {}",
        devices.len(),
        router.mode()
    );

    let mut pollfds: Vec<libc::pollfd> = devices
        .iter()
        .map(|d| libc::pollfd {
            fd: d.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        })
        .collect();

    loop {
        let timeout = match router.deadline() {
            Some(dl) => (dl.saturating_sub(ms(&start))) as i32,
            None => -1,
        };
        for p in &mut pollfds {
            p.revents = 0;
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
            execute(&mut vdev, &mut hypr, router.on_timeout(ms(&start)));
            continue;
        }
        for i in 0..devices.len() {
            if pollfds[i].revents & libc::POLLIN == 0 {
                continue;
            }
            let events: Vec<(u16, i32)> = match devices[i].fetch_events() {
                Ok(it) => it
                    .filter(|e| e.event_type() == EventType::KEY)
                    .map(|e| (e.code(), e.value()))
                    .collect(),
                Err(_) => continue,
            };
            for (code, value) in events {
                let fx = router.on_key(code, value, ms(&start));
                execute(&mut vdev, &mut hypr, fx);
            }
        }
        let _ = InputEvent::new; // keep import meaningful across cfgs
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
            Effect::ModeChanged(mode) => log::info!("mode → {mode}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
      "superCtrlRemaps": { "copy": { "from": "super + c", "to": "ctrl + c" } },
      "modes": {
        "app": { "bindings": {
          "ws1": { "key": "super + 1", "action": "workspace, 1" },
          "enterDesktopHold": { "key": "F23", "action": "submap, desktop" }
        }},
        "desktop": { "bindings": {
          "focusLeft": { "key": "h", "action": "movefocus, l", "repeat": true },
          "enterMove": { "key": "m", "action": "submap, move" },
          "close": { "key": "q", "action": "killactive,", "exitAfter": true }
        }},
        "move": { "bindings": {
          "moveLeft": { "key": "h", "action": "movewindow, l", "repeat": true }
        }},
        "resize": { "bindings": {} },
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
    fn super_number_dispatches_workspace_not_passthrough() {
        let mut r = router();
        r.on_key(SUPER, 1, 0);
        let fx = r.on_key(KeyCode::KEY_1.0, 1, 10);
        assert!(fx.contains(&Effect::Dispatch("workspace, 1".into())));
    }

    #[test]
    fn app_mode_passes_through_typing() {
        let mut r = router();
        // plain 'a' in app mode → emitted (typing works).
        let fx = r.on_key(A, 1, 0);
        assert_eq!(fx, vec![Effect::Emit { code: A, value: 1 }]);
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

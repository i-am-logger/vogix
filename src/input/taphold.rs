//! CapsLock tap/hold detector — the only real-time timing in the input engine,
//! isolated here so it is tested *deterministically*: time is an input
//! (millisecond timestamps fed in), never a side effect.
//!
//! # Model (Raskin 2000 quasimode + QMK/ZMK "tap-hold-press")
//!
//! - **Hold** CapsLock — or press any other key while it is down — activates a
//!   *momentary* mode: enter on press, leave on release. This is Raskin's
//!   quasimode (The Humane Interface §3-2): a mode you cannot get stranded in,
//!   because letting go always leaves it.
//! - **Tap** CapsLock alone (released within `tap_hold_ms`) toggles the *sticky*
//!   (locked) mode on/off.
//!
//! The "tap-hold-**press**" rule (QMK/ZMK firmware practice) is the key subtlety
//! that the kanata config also relied on: the hold resolves the *instant* another
//! key goes down, so "caps + key" is always momentary and can never mis-resolve
//! as a tap. This was the seam where the old two-system setup leaked the
//! "stuck in a mode" bug; here it is one state machine with proven invariants.
//!
//! The detector emits abstract [`CapsIntent`]s. Mapping them to engine
//! [`ModeTransition`](super::engine::ModeTransition)s (and deciding sticky
//! on/off from the engine's current state) is the device loop's job — the
//! detector stays pure and stateless about modes.

/// A raw CapsLock-related event, timestamped in milliseconds (monotonic).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapsEvent {
    /// CapsLock pressed.
    CapsDown(u64),
    /// CapsLock released.
    CapsUp(u64),
    /// Some *other* key was pressed (resolves a pending caps-hold immediately).
    OtherKeyDown(u64),
    /// The device loop's poll deadline elapsed; carries the current time.
    Timeout(u64),
}

/// What the detector decided a gesture means.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapsIntent {
    /// A momentary mode began (caps held / caps+key). → `EnterMomentary`.
    HoldStart,
    /// The momentary mode ended (caps released). → `ReleaseHold`.
    HoldEnd,
    /// A lone click. → toggle the sticky mode (engine decides on vs off).
    Tap,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    /// CapsLock is up; nothing pending.
    Idle,
    /// CapsLock is down, not yet resolved as tap or hold.
    Pending { down_at: u64 },
    /// Resolved as a hold (momentary mode active).
    Holding,
}

/// The tap/hold state machine. Robust to redundant/duplicated events.
#[derive(Debug, Clone)]
pub struct CapsDetector {
    tap_hold_ms: u64,
    state: State,
}

impl CapsDetector {
    /// Create a detector with the tap↔hold threshold (the deployed value is 250ms).
    pub fn new(tap_hold_ms: u64) -> Self {
        Self {
            tap_hold_ms,
            state: State::Idle,
        }
    }

    /// When (in the same clock as the events) the loop should deliver a
    /// [`CapsEvent::Timeout`] to let a pure hold resolve. `None` when nothing is
    /// pending. The loop polls input with this as its deadline.
    pub fn deadline(&self) -> Option<u64> {
        match self.state {
            State::Pending { down_at } => Some(down_at + self.tap_hold_ms),
            _ => None,
        }
    }

    /// Feed one event; returns the intent it resolved, if any.
    pub fn feed(&mut self, event: CapsEvent) -> Option<CapsIntent> {
        match (self.state, event) {
            // ── Idle ──
            (State::Idle, CapsEvent::CapsDown(t)) => {
                self.state = State::Pending { down_at: t };
                None
            }
            // Stray events while idle are not ours.
            (State::Idle, _) => None,

            // ── Pending (caps down, undecided) ──
            // Another key pressed → resolve as hold immediately (tap-hold-press).
            (State::Pending { .. }, CapsEvent::OtherKeyDown(_)) => {
                self.state = State::Holding;
                Some(CapsIntent::HoldStart)
            }
            // Held past the threshold with no other key → resolve as hold.
            (State::Pending { down_at }, CapsEvent::Timeout(t)) => {
                if t >= down_at + self.tap_hold_ms {
                    self.state = State::Holding;
                    Some(CapsIntent::HoldStart)
                } else {
                    None
                }
            }
            // Released before resolving → it was a tap.
            (State::Pending { .. }, CapsEvent::CapsUp(_)) => {
                self.state = State::Idle;
                Some(CapsIntent::Tap)
            }
            // Redundant caps-down while pending — ignore.
            (State::Pending { .. }, CapsEvent::CapsDown(_)) => None,

            // ── Holding (momentary mode active) ──
            (State::Holding, CapsEvent::CapsUp(_)) => {
                self.state = State::Idle;
                Some(CapsIntent::HoldEnd)
            }
            // Extra keys / redundant downs / timeouts while holding — no new intent.
            (State::Holding, _) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    const MS: u64 = 250;

    /// Drive a detector through a sequence, collecting intents.
    fn run(events: impl IntoIterator<Item = CapsEvent>) -> Vec<CapsIntent> {
        let mut d = CapsDetector::new(MS);
        events.into_iter().filter_map(|e| d.feed(e)).collect()
    }

    #[test]
    fn lone_short_click_is_a_tap() {
        let out = run([CapsEvent::CapsDown(0), CapsEvent::CapsUp(50)]);
        assert_eq!(out, vec![CapsIntent::Tap]);
    }

    #[test]
    fn caps_plus_key_is_momentary_hold() {
        // caps down, another key 10ms later (< 250), then caps up → hold pair.
        let out = run([
            CapsEvent::CapsDown(0),
            CapsEvent::OtherKeyDown(10),
            CapsEvent::CapsUp(80),
        ]);
        assert_eq!(out, vec![CapsIntent::HoldStart, CapsIntent::HoldEnd]);
    }

    #[test]
    fn caps_plus_key_never_mis_resolves_as_tap() {
        // Even though caps was released quickly (80ms < 250), pressing a key
        // first makes it a hold — the tap-hold-press rule. No Tap may appear.
        let out = run([
            CapsEvent::CapsDown(0),
            CapsEvent::OtherKeyDown(5),
            CapsEvent::CapsUp(80),
        ]);
        assert!(!out.contains(&CapsIntent::Tap), "got {out:?}");
    }

    #[test]
    fn long_hold_with_no_key_resolves_on_timeout() {
        let out = run([
            CapsEvent::CapsDown(0),
            CapsEvent::Timeout(250),
            CapsEvent::CapsUp(600),
        ]);
        assert_eq!(out, vec![CapsIntent::HoldStart, CapsIntent::HoldEnd]);
    }

    #[test]
    fn timeout_before_threshold_does_not_resolve() {
        // A poll wake-up before the deadline must not prematurely start a hold.
        let mut d = CapsDetector::new(MS);
        assert_eq!(d.feed(CapsEvent::CapsDown(0)), None);
        assert_eq!(d.feed(CapsEvent::Timeout(100)), None);
        assert_eq!(d.feed(CapsEvent::CapsUp(120)), Some(CapsIntent::Tap));
    }

    #[test]
    fn deadline_tracks_pending_state() {
        let mut d = CapsDetector::new(MS);
        assert_eq!(d.deadline(), None);
        d.feed(CapsEvent::CapsDown(40));
        assert_eq!(d.deadline(), Some(40 + MS));
        d.feed(CapsEvent::OtherKeyDown(50));
        assert_eq!(d.deadline(), None, "no deadline once holding");
    }

    #[test]
    fn redundant_events_are_ignored() {
        let out = run([
            CapsEvent::CapsDown(0),
            CapsEvent::CapsDown(1), // duplicate down
            CapsEvent::OtherKeyDown(10),
            CapsEvent::OtherKeyDown(20), // more keys while holding
            CapsEvent::CapsUp(50),
        ]);
        assert_eq!(out, vec![CapsIntent::HoldStart, CapsIntent::HoldEnd]);
    }

    // ── Property tests ──

    /// Build a well-formed-ish event stream with strictly monotonic timestamps.
    fn event_stream() -> impl Strategy<Value = Vec<CapsEvent>> {
        proptest::collection::vec((0u8..4, 1u64..400), 0..60).prop_map(|steps| {
            let mut t = 0u64;
            steps
                .into_iter()
                .map(|(kind, dt)| {
                    t += dt;
                    match kind {
                        0 => CapsEvent::CapsDown(t),
                        1 => CapsEvent::CapsUp(t),
                        2 => CapsEvent::OtherKeyDown(t),
                        _ => CapsEvent::Timeout(t),
                    }
                })
                .collect()
        })
    }

    proptest! {
        /// THE anti-stuck invariant at the timing layer: HoldStart/HoldEnd are
        /// perfectly nested — depth only ever 0 or 1, and it ends balanced once
        /// CapsLock is up. A momentary mode can never be entered without a
        /// matching exit, so the detector cannot strand the engine in a mode.
        #[test]
        fn prop_holds_are_balanced(events in event_stream()) {
            let mut d = CapsDetector::new(MS);
            let mut depth: i32 = 0;
            let mut caps_down = false;
            for e in &events {
                match e {
                    CapsEvent::CapsDown(_) => caps_down = true,
                    CapsEvent::CapsUp(_) => caps_down = false,
                    _ => {}
                }
                if let Some(intent) = d.feed(*e) {
                    match intent {
                        CapsIntent::HoldStart => depth += 1,
                        CapsIntent::HoldEnd => depth -= 1,
                        CapsIntent::Tap => {}
                    }
                }
                prop_assert!((0..=1).contains(&depth), "hold depth escaped {{0,1}}: {depth}");
            }
            // When CapsLock is not physically down, we cannot be mid-hold.
            if !caps_down {
                prop_assert_eq!(depth, 0, "left in a hold while caps is up");
            }
        }

        /// A Tap is only ever produced by releasing caps before it resolved to a
        /// hold — never once a hold has started. (Tap and Hold are exclusive per
        /// gesture.)
        #[test]
        fn prop_tap_excludes_hold_within_a_gesture(events in event_stream()) {
            let mut d = CapsDetector::new(MS);
            let mut holding = false;
            for e in &events {
                if let Some(intent) = d.feed(*e) {
                    match intent {
                        CapsIntent::HoldStart => holding = true,
                        CapsIntent::HoldEnd => holding = false,
                        CapsIntent::Tap => prop_assert!(!holding, "tap emitted during a hold"),
                    }
                }
            }
        }
    }
}

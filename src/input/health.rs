//! Non-keylogging observability for the input engine.
//!
//! The engine was a black box: app-mode re-emit logs at TRACE, so at the deployed
//! debug level you cannot tell whether keystrokes reach the engine — the exact
//! confusion of the flaky-keyboard incident (was a key swallowed by a mode, or
//! never delivered by the hardware?). Promoting re-emit to DEBUG would keylog
//! journald. So this surface gives flow visibility WITHOUT key identity: per-
//! device COUNTS, a stuck-key COUNT, and device *hardware* identity — never which
//! key was pressed.
//!
//! The engine writes a [`HealthSnapshot`] to
//! `~/.local/state/vogix/input-health.json` (best-effort, atomic tmp+rename —
//! mirroring `publish_mode`/`current-mode`); `vogix input doctor` renders it.
//! "Keychron flowing, Logitech silent → hardware" becomes a one-command read.

use super::device::Effect;
use crate::config::Config;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// A grabbed device's stable identity, captured once at grab time (the evdev
/// `Device` is borrowed by the poll loop afterwards, so we cannot re-read it).
#[derive(Debug, Clone)]
pub struct DeviceMeta {
    pub name: String,
    pub vendor: u16,
    pub product: u16,
}

/// Per-device event-flow counters. Monotonic; carry NO key identity — only
/// tallies and a last-seen timestamp.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Counters {
    pub events_in: u64,
    pub emitted: u64,
    pub dispatched: u64,
    pub keyword: u64,
    pub mode_changes: u64,
    /// Events the engine consumed with no visible effect (catchall swallow, a
    /// tracked modifier, an unmapped Super-combo) — i.e. an empty effect list.
    pub swallowed: u64,
    /// Engine-clock ms of the last event from this device (0 = none yet).
    pub last_event_ms: u64,
}

impl Counters {
    /// Account one fetched `(code,value)` from this device and the effects the
    /// Router produced for it. Records NO key identity — only counts.
    pub fn record(&mut self, fx: &[Effect], now: u64) {
        self.events_in += 1;
        self.last_event_ms = now;
        if fx.is_empty() {
            self.swallowed += 1;
        }
        for e in fx {
            match e {
                Effect::Emit { .. } => self.emitted += 1,
                Effect::Dispatch(_) => self.dispatched += 1,
                Effect::Keyword { .. } => self.keyword += 1,
                Effect::ModeChanged(_) => self.mode_changes += 1,
            }
        }
    }
}

/// Tracks which keys are currently held, to detect a STUCK key (down with no up).
/// The keycode is used ONLY to pair down/up events and is NEVER serialised or
/// logged — only the COUNT and the oldest age leave this type.
#[derive(Debug, Default)]
pub struct HeldKeys {
    down: HashMap<u16, u64>,
}

impl HeldKeys {
    pub fn on_event(&mut self, code: u16, value: i32, now: u64) {
        match value {
            1 => {
                self.down.entry(code).or_insert(now);
            }
            0 => {
                self.down.remove(&code);
            }
            _ => {} // auto-repeat (value 2): not a new press
        }
    }

    /// `(count, oldest_age_ms)` of keys held longer than `threshold_ms`.
    pub fn stuck(&self, now: u64, threshold_ms: u64) -> (usize, u64) {
        let mut count = 0usize;
        let mut oldest = 0u64;
        for &down_at in self.down.values() {
            let age = now.saturating_sub(down_at);
            if age >= threshold_ms {
                count += 1;
                oldest = oldest.max(age);
            }
        }
        (count, oldest)
    }
}

/// One grabbed device's identity + flow counts in a snapshot. NO key identity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceHealth {
    pub name: String,
    pub vendor: u16,
    pub product: u16,
    #[serde(flatten)]
    pub counters: Counters,
    /// ms since this device's last event, at snapshot time (the "went silent"
    /// tell — the signal that localised the flaky keyboard to the hardware).
    pub silent_ms: u64,
}

/// A point-in-time health snapshot the engine writes for `vogix input doctor`.
/// Device hardware identity + aggregate counts only — never key identity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthSnapshot {
    pub pid: u32,
    pub uptime_ms: u64,
    pub mode: String,
    pub devices: Vec<DeviceHealth>,
    pub stuck_count: usize,
    pub stuck_oldest_ms: u64,
}

/// Keys held this long with no key-up are reported as stuck (well past
/// auto-repeat and any human hold).
pub const STUCK_MS: u64 = 5000;

fn health_path() -> PathBuf {
    Config::state_dir().join("input-health.json")
}

/// Best-effort atomic write (tmp + rename), like `publish_mode`. A failure to
/// write diagnostics must NEVER affect input, so all errors are swallowed.
pub fn write_snapshot(snap: &HealthSnapshot) {
    let Ok(json) = serde_json::to_string_pretty(snap) else {
        return;
    };
    let dir = Config::state_dir();
    // Ensure the state dir exists: when the engine loads its schema from an
    // explicit --config path it never otherwise touches the state dir, so a
    // fresh system would have no ~/.local/state/vogix to write into.
    let _ = std::fs::create_dir_all(&dir);
    let tmp = dir.join("input-health.json.tmp");
    if std::fs::write(&tmp, json).is_ok() {
        let _ = std::fs::rename(&tmp, dir.join("input-health.json"));
    }
}

/// Read the snapshot for `vogix input doctor`. `None` if absent or unparseable
/// (which itself diagnoses a never-started / crashed / outdated engine).
pub fn read_snapshot() -> Option<HealthSnapshot> {
    let text = std::fs::read_to_string(health_path()).ok()?;
    serde_json::from_str(&text).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counters_classify_effects_without_key_identity() {
        let mut c = Counters::default();
        c.record(&[Effect::Emit { code: 30, value: 1 }], 10);
        c.record(&[Effect::Dispatch("workspace, 1".into())], 20);
        c.record(&[], 30); // swallowed (empty effect list)
        assert_eq!(c.events_in, 3);
        assert_eq!(c.emitted, 1);
        assert_eq!(c.dispatched, 1);
        assert_eq!(c.swallowed, 1);
        assert_eq!(c.last_event_ms, 30);
    }

    #[test]
    fn a_remap_emitting_several_keys_counts_one_event_in() {
        // One physical key → a 4-event Ctrl+C chord: events_in stays 1.
        let mut c = Counters::default();
        c.record(
            &[
                Effect::Emit { code: 29, value: 1 },
                Effect::Emit { code: 46, value: 1 },
                Effect::Emit { code: 46, value: 0 },
                Effect::Emit { code: 29, value: 0 },
            ],
            5,
        );
        assert_eq!(c.events_in, 1);
        assert_eq!(c.emitted, 4);
        assert_eq!(c.swallowed, 0);
    }

    #[test]
    fn stuck_key_detected_after_threshold_then_cleared_on_up() {
        let mut h = HeldKeys::default();
        h.on_event(30, 1, 0); // A down at t=0
        assert_eq!(h.stuck(100, STUCK_MS), (0, 0), "not yet stuck");
        assert_eq!(h.stuck(6000, STUCK_MS).0, 1, "held past threshold → stuck");
        assert!(h.stuck(6000, STUCK_MS).1 >= STUCK_MS);
        h.on_event(30, 2, 5500); // auto-repeat does NOT add a second entry
        assert_eq!(h.stuck(6000, STUCK_MS).0, 1);
        h.on_event(30, 0, 6100); // up clears it
        assert_eq!(h.stuck(7000, STUCK_MS), (0, 0));
    }

    #[test]
    fn snapshot_json_carries_no_keycode_identity() {
        let snap = HealthSnapshot {
            pid: 1234,
            uptime_ms: 5000,
            mode: "app".into(),
            devices: vec![DeviceHealth {
                name: "Keychron K2 HE Keyboard".into(),
                vendor: 0x3434,
                product: 0x0e20,
                counters: Counters {
                    events_in: 42,
                    emitted: 40,
                    last_event_ms: 4900,
                    ..Default::default()
                },
                silent_ms: 100,
            }],
            stuck_count: 0,
            stuck_oldest_ms: 0,
        };
        let json = serde_json::to_string(&snap).unwrap();
        // Counts and device identity are present…
        assert!(json.contains("events_in"));
        assert!(json.contains("Keychron"));
        // …but no per-key identity field leaks (the no-keylog invariant).
        assert!(!json.contains("\"code\""));
        assert!(!json.contains("\"key\""));
        // Round-trips.
        let back: HealthSnapshot = serde_json::from_str(&json).unwrap();
        assert_eq!(back.devices[0].counters.events_in, 42);
    }
}

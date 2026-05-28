//! Ontology-driven input engine — the kanata + Hyprland-submap replacement.
//!
//! Keybindings used to be split across two systems with two models: kanata
//! (tap-hold → keysym) and Hyprland (submaps + release-binds). The gaps between
//! them (submap straddle, dropped fake-key taps, release-binds that never fire)
//! produced the recurring "stuck in a mode" bug, which no patch fully closed.
//!
//! This module collapses both into ONE engine, driven by *loaded config* (the
//! schema), not hard-coded — the way praxis loads a linguistic ontology rather
//! than baking in English. The plan:
//!   - [`schema`] — loads the keybinding ontology (mode graph + bindings) from
//!     config (`defaults.nix` → JSON) and derives the praxis mode graph from it.
//!   - [`engine`] — the generic mode statechart dynamics, with the no-stuck
//!     guarantee proven by property tests (relocated from praxis; see its docs).
//!   - [`taphold`] — the CapsLock tap/hold detector (the only real-time timing).
//!   - (Phase 2b) device layer: grab evdev, run the statechart in-process,
//!     dispatch window actions to Hyprland's IPC socket and re-emit normal keys
//!     via uinput. No kanata, no submaps, no F22/F23/F24 bridge.
//!
//! caps↓ enters a (validated) mode; caps↑ is `ReleaseHold`, which is always
//! legal. "Stuck" is not a bug to fix here — it is an unrepresentable state.

pub mod engine;
pub mod schema;
// Consumed by the device loop (next 2b step); kept allow(dead_code) until wired.
#[allow(dead_code)]
pub mod hypr;
#[allow(dead_code)]
pub mod keys;
#[allow(dead_code)]
pub mod taphold;

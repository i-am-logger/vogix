//! Ontology-driven input engine — the kanata + Hyprland-submap replacement.
//!
//! Keybindings used to be split across two systems with two models: kanata
//! (tap-hold → keysym) and Hyprland (submaps + release-binds). The gaps between
//! them (submap straddle, dropped fake-key taps, release-binds that never fire)
//! produced the recurring "stuck in a mode" bug, which no patch fully closed.
//!
//! This module collapses both into ONE engine, driven by *loaded config* (the
//! schema), not hard-coded — the way praxis loads a linguistic ontology rather
//! than baking in English.
//!
//! The mode *ontology* and statechart dynamics — `ModeGraph`/`ModeId`,
//! `InputState`, `ModeTransition`, `new_input_engine`/`drive`, and the no-stuck
//! axioms (proven by property tests over arbitrary graphs) — live in praxis
//! (`pr4xis_domains::applied::hmi::input::{modes,engine,ontology}`). This module
//! is the *runtime / I/O* layer that consumes them:
//!   - [`schema`] — loads the keybinding ontology (mode graph + bindings) from
//!     config (`defaults.nix` → JSON) and derives the praxis mode graph from it.
//!   - [`taphold`] — the CapsLock tap/hold detector (the only real-time timing).
//!   - [`keys`] — evdev keycode ↔ logical chord mapping.
//!   - [`hypr`] — best-effort compositor control over its IPC socket.
//!   - [`device`] — the device layer: grabs evdev, drives the praxis statechart
//!     in-process, dispatches window actions to the compositor's IPC socket and
//!     re-emits normal keys via uinput. No kanata, no submaps, no F22/F23/F24
//!     bridge.
//!
//! caps↓ enters a (validated) mode; caps↑ is `ReleaseHold`, which is always
//! legal. "Stuck" is not a bug to fix here — it is an unrepresentable state.

pub mod catalog;
pub mod devfilter;
pub mod device;
pub mod health;
pub mod hypr;
pub mod keys;
pub mod paradigm;
pub mod schema;
pub mod taphold;

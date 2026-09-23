//! Machine surfaces: state that belongs to the machine rather than to a user
//! session — LEDs behind the OpenRGB server, command-driven devices and the
//! kernel's VT palette.
//!
//! One declared owner publishes `palette.json` into a drop zone; system units
//! read it together with `/etc/vogix/machine.json` and apply it. This module
//! holds the data model both sides share, the publisher, and the owners.

pub mod command;
pub mod config;
pub mod console;
pub mod exit;
pub mod local;
pub mod notify;
pub mod openrgb;
pub mod palette;
pub mod publish;
pub mod reactor;
pub mod status;
pub mod types;
pub mod uevent;

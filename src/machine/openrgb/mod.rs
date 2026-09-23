//! vogix's OpenRGB SDK client.
//!
//! Written from the SDK's documented wire format (`Documentation/OpenRGBSDK.md`)
//! and checked against the server source vogix's OpenRGB package is built from;
//! it speaks protocols 5 and 6 and uses only std and serde.
//!
//! - [`wire`]: packet framing, packet ids, bounds-checked readers and writers,
//!   and the primitive string and colour types.
//! - [`model`]: the negotiated version, flag and enum types, and controller
//!   descriptions.
//! - [`codec`]: version-gated decoders for every block and reply vogix reads,
//!   and encoders for the packets it sends.
//! - [`select`]: device selection, the server's mode acceptance rule, write
//!   plans and read-back confirmation.
//! - [`session`]: the connection state machine that drives them, performing no
//!   I/O of its own.

pub mod codec;
pub mod model;
pub mod select;
pub mod session;
pub mod wire;

#[cfg(test)]
mod testkit;

//! OpenRGB SDK framing and primitive wire types.
//!
//! Every SDK packet is a 16-byte header — the magic `"ORGB"`, then `dev_id`,
//! `pkt_id` and `pkt_size` as little-endian `u32` — followed by `pkt_size`
//! bytes of payload (`NetworkProtocol.h`, `NetPacketHeader`). This module owns
//! that framing, the typed [`PacketId`], an incremental [`Framer`] for a byte
//! stream, and the bounds-checked [`Reader`]/[`Writer`] the codec builds on.
//!
//! Strings inside description blocks carry a length prefix that counts the
//! terminating NUL; [`WireString`] keeps the bytes before that NUL verbatim, so
//! a decoded block re-encodes to the same bytes. Colours are OpenRGB's
//! `RGBColor`, a `u32` packed `R | G << 8 | B << 16` ([`RgbColor`]).

use serde::{Serialize, Serializer};
use std::borrow::Cow;
use std::fmt;

/// The packet magic, `"ORGB"`.
pub const MAGIC: [u8; 4] = *b"ORGB";

/// Size of the packet header in bytes.
pub const HEADER_LEN: usize = 16;

/// `OPENRGB_SDK_MAX_PACKET_SIZE` (8 MiB, `NetworkProtocol.h`). The server closes a
/// client that sends a larger packet; vogix treats a larger incoming packet as a
/// protocol violation and never sends one.
pub const MAX_PACKET_SIZE: u32 = 8 * 1024 * 1024;

/// SDK packet ids vogix sends or interprets. Every other id decodes to
/// [`PacketId::Unknown`] and is skipped by its declared size.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum PacketId {
    /// 0: controller count (v5) or count plus unique ids (v6).
    RequestControllerCount,
    /// 1: one controller's description block.
    RequestControllerData,
    /// 10: acknowledgement of a client packet (v6).
    Ack,
    /// 40: protocol version exchange.
    RequestProtocolVersion,
    /// 50: client name, a NUL-terminated string.
    SetClientName,
    /// 51: server name, a NUL-terminated string (v6).
    SetServerName,
    /// 52: client capability flags (v6).
    SetClientFlags,
    /// 53: server capability flags (v6).
    SetServerFlags,
    /// 100: the server's controller list changed.
    DeviceListUpdated,
    /// 101: detection started (v6).
    DetectionStarted,
    /// 102: detection progress (v6).
    DetectionProgressChanged,
    /// 103: detection completed (v6).
    DetectionComplete,
    /// 1050: `RGBController::UpdateLEDs()` with new colours.
    UpdateLeds,
    /// 1101: `RGBController::UpdateMode()` with a mode block.
    UpdateMode,
    /// 1150: `RGBController::SignalUpdate()` notification (v6).
    SignalUpdate,
    /// Any other id; the value is never one of the ids above.
    Unknown(u32),
}

impl PacketId {
    /// Map a wire id to its typed form.
    pub const fn from_wire(id: u32) -> Self {
        match id {
            0 => Self::RequestControllerCount,
            1 => Self::RequestControllerData,
            10 => Self::Ack,
            40 => Self::RequestProtocolVersion,
            50 => Self::SetClientName,
            51 => Self::SetServerName,
            52 => Self::SetClientFlags,
            53 => Self::SetServerFlags,
            100 => Self::DeviceListUpdated,
            101 => Self::DetectionStarted,
            102 => Self::DetectionProgressChanged,
            103 => Self::DetectionComplete,
            1050 => Self::UpdateLeds,
            1101 => Self::UpdateMode,
            1150 => Self::SignalUpdate,
            other => Self::Unknown(other),
        }
    }

    /// The wire id.
    pub const fn to_wire(self) -> u32 {
        match self {
            Self::RequestControllerCount => 0,
            Self::RequestControllerData => 1,
            Self::Ack => 10,
            Self::RequestProtocolVersion => 40,
            Self::SetClientName => 50,
            Self::SetServerName => 51,
            Self::SetClientFlags => 52,
            Self::SetServerFlags => 53,
            Self::DeviceListUpdated => 100,
            Self::DetectionStarted => 101,
            Self::DetectionProgressChanged => 102,
            Self::DetectionComplete => 103,
            Self::UpdateLeds => 1050,
            Self::UpdateMode => 1101,
            Self::SignalUpdate => 1150,
            Self::Unknown(id) => id,
        }
    }
}

impl fmt::Display for PacketId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unknown(id) => write!(f, "packet {id}"),
            known => write!(f, "{known:?} ({})", known.to_wire()),
        }
    }
}

impl Serialize for PacketId {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_u32(self.to_wire())
    }
}

/// A decoded packet header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Header {
    pub dev_id: u32,
    pub pkt_id: PacketId,
    pub size: u32,
}

impl Header {
    /// The 16 header bytes.
    pub fn encode(self) -> [u8; HEADER_LEN] {
        let mut bytes = [0u8; HEADER_LEN];
        bytes[0..4].copy_from_slice(&MAGIC);
        bytes[4..8].copy_from_slice(&self.dev_id.to_le_bytes());
        bytes[8..12].copy_from_slice(&self.pkt_id.to_wire().to_le_bytes());
        bytes[12..16].copy_from_slice(&self.size.to_le_bytes());
        bytes
    }

    /// Parse a header, rejecting a wrong magic or a size above
    /// [`MAX_PACKET_SIZE`].
    pub fn decode(bytes: &[u8; HEADER_LEN]) -> Result<Self, FramingError> {
        let magic = [bytes[0], bytes[1], bytes[2], bytes[3]];
        if magic != MAGIC {
            return Err(FramingError::BadMagic(magic));
        }
        let word = |at: usize| {
            u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
        };
        let header = Self {
            dev_id: word(4),
            pkt_id: PacketId::from_wire(word(8)),
            size: word(12),
        };
        if header.size > MAX_PACKET_SIZE {
            return Err(FramingError::Oversize {
                pkt_id: header.pkt_id,
                size: header.size,
            });
        }
        Ok(header)
    }
}

/// One complete packet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    pub dev_id: u32,
    pub pkt_id: PacketId,
    pub payload: Vec<u8>,
}

impl Frame {
    /// Header plus payload bytes. Fails only when the payload exceeds
    /// [`MAX_PACKET_SIZE`].
    pub fn encode(&self) -> Result<Vec<u8>, EncodeError> {
        let size = payload_size(self.payload.len())?;
        let header = Header {
            dev_id: self.dev_id,
            pkt_id: self.pkt_id,
            size,
        };
        let mut bytes = Vec::with_capacity(HEADER_LEN + self.payload.len());
        bytes.extend_from_slice(&header.encode());
        bytes.extend_from_slice(&self.payload);
        Ok(bytes)
    }
}

/// A payload length as the header's `u32`, bounded by [`MAX_PACKET_SIZE`].
pub fn payload_size(len: usize) -> Result<u32, EncodeError> {
    u32::try_from(len)
        .ok()
        .filter(|size| *size <= MAX_PACKET_SIZE)
        .ok_or(EncodeError::PacketTooLarge { size: len })
}

/// The byte stream cannot be framed any further.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum FramingError {
    #[error("packet magic is {0:02x?}, not \"ORGB\"")]
    BadMagic([u8; 4]),
    #[error("{pkt_id} declares {size} bytes, above the {MAX_PACKET_SIZE}-byte SDK packet limit")]
    Oversize { pkt_id: PacketId, size: u32 },
}

/// Incremental framer over a byte stream.
///
/// Bytes arrive in arbitrary chunks through [`Framer::push`];
/// [`Framer::next_frame`] yields each complete packet in order. A header with a
/// wrong magic or an oversize length breaks the stream for good: every later
/// call returns the same error and pushed bytes are dropped, because nothing
/// after a framing error can be located reliably.
#[derive(Debug, Default)]
pub struct Framer {
    buf: Vec<u8>,
    start: usize,
    header: Option<Header>,
    broken: Option<FramingError>,
}

impl Framer {
    pub fn new() -> Self {
        Self::default()
    }

    /// Append bytes read from the stream.
    pub fn push(&mut self, bytes: &[u8]) {
        if self.broken.is_none() {
            self.buf.extend_from_slice(bytes);
        }
    }

    /// The next complete packet, `Ok(None)` when more bytes are needed.
    pub fn next_frame(&mut self) -> Result<Option<Frame>, FramingError> {
        if let Some(error) = self.broken {
            return Err(error);
        }
        let header = match self.header {
            Some(header) => header,
            None => {
                let Some(raw) = self.buf.get(self.start..self.start + HEADER_LEN) else {
                    return Ok(None);
                };
                let mut bytes = [0u8; HEADER_LEN];
                bytes.copy_from_slice(raw);
                match Header::decode(&bytes) {
                    Ok(header) => {
                        self.start += HEADER_LEN;
                        self.header = Some(header);
                        header
                    }
                    Err(error) => {
                        self.broken = Some(error);
                        self.buf = Vec::new();
                        self.start = 0;
                        return Err(error);
                    }
                }
            }
        };
        let size = header.size as usize;
        let Some(payload) = self.buf.get(self.start..self.start + size) else {
            return Ok(None);
        };
        let frame = Frame {
            dev_id: header.dev_id,
            pkt_id: header.pkt_id,
            payload: payload.to_vec(),
        };
        self.start += size;
        self.header = None;
        self.compact();
        Ok(Some(frame))
    }

    /// Bytes held that do not yet form a complete packet.
    pub fn buffered(&self) -> usize {
        self.buf.len() - self.start
    }

    fn compact(&mut self) {
        if self.start == self.buf.len() {
            self.buf.clear();
            self.start = 0;
        } else if self.start >= self.buf.len() / 2 {
            self.buf.drain(..self.start);
            self.start = 0;
        }
    }
}

/// A string from a description block: the bytes before the terminating NUL.
///
/// OpenRGB writes these with `strcpy`, so the stored length is `strlen + 1` and
/// the last byte is NUL. The bytes are kept as-is (they are not required to be
/// UTF-8) so a decoded block re-encodes byte for byte.
#[derive(Clone, PartialEq, Eq, Hash, Default, PartialOrd, Ord)]
pub struct WireString(Vec<u8>);

impl WireString {
    /// Wrap bytes that exclude the terminating NUL.
    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        Self(bytes)
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    /// The text, with invalid UTF-8 replaced.
    pub fn to_string_lossy(&self) -> Cow<'_, str> {
        String::from_utf8_lossy(&self.0)
    }

    /// ASCII case-insensitive substring test.
    pub fn contains_ignore_ascii_case(&self, needle: &str) -> bool {
        let needle = needle.as_bytes();
        if needle.is_empty() {
            return true;
        }
        self.0
            .windows(needle.len())
            .any(|window| window.eq_ignore_ascii_case(needle))
    }

    /// ASCII case-insensitive equality.
    pub fn eq_ignore_ascii_case(&self, other: &str) -> bool {
        self.0.eq_ignore_ascii_case(other.as_bytes())
    }
}

impl From<&str> for WireString {
    fn from(text: &str) -> Self {
        Self(text.as_bytes().to_vec())
    }
}

impl fmt::Debug for WireString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.to_string_lossy())
    }
}

impl fmt::Display for WireString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_string_lossy())
    }
}

impl Serialize for WireString {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string_lossy())
    }
}

/// OpenRGB's `RGBColor`: `R | G << 8 | B << 16` in a little-endian `u32`
/// (`RGBControllerInterface.h`, `ToRGBColor`). The top byte is kept as
/// received so a decoded block re-encodes byte for byte.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct RgbColor(u32);

impl RgbColor {
    pub const fn from_rgb(red: u8, green: u8, blue: u8) -> Self {
        Self(red as u32 | (green as u32) << 8 | (blue as u32) << 16)
    }

    pub const fn from_wire(value: u32) -> Self {
        Self(value)
    }

    pub const fn to_wire(self) -> u32 {
        self.0
    }

    pub const fn red(self) -> u8 {
        (self.0 & 0xff) as u8
    }

    pub const fn green(self) -> u8 {
        ((self.0 >> 8) & 0xff) as u8
    }

    pub const fn blue(self) -> u8 {
        ((self.0 >> 16) & 0xff) as u8
    }
}

impl fmt::Display for RgbColor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "#{:02x}{:02x}{:02x}",
            self.red(),
            self.green(),
            self.blue()
        )?;
        let top = self.0 >> 24;
        if top != 0 {
            write!(f, " (top byte {top:#04x})")?;
        }
        Ok(())
    }
}

impl fmt::Debug for RgbColor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl Serialize for RgbColor {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

/// A payload could not be decoded.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{field}: {kind}")]
pub struct DecodeError {
    /// The field being read when decoding failed.
    pub field: &'static str,
    pub kind: DecodeErrorKind,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DecodeErrorKind {
    #[error("needs {needed} bytes, {remaining} remain")]
    Truncated { needed: usize, remaining: usize },
    #[error("string has no terminating NUL")]
    MissingNul,
    #[error("{0} unexpected trailing bytes")]
    TrailingBytes(usize),
    #[error("declared data size {declared} but the packet carries {actual} bytes")]
    SizeMismatch { declared: u32, actual: usize },
    #[error("controller id {0} listed twice")]
    DuplicateId(u32),
    #[error("matrix {height}x{width} does not fit the packet")]
    MatrixTooLarge { height: u32, width: u32 },
}

impl DecodeError {
    pub fn new(field: &'static str, kind: DecodeErrorKind) -> Self {
        Self { field, kind }
    }
}

/// A value could not be encoded.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum EncodeError {
    #[error("{field} is {len} bytes, above the {max}-byte limit of its length field")]
    StringTooLong {
        field: &'static str,
        len: usize,
        max: u64,
    },
    #[error("{field} has {count} entries, above the {max} its count field holds")]
    TooManyItems {
        field: &'static str,
        count: usize,
        max: u64,
    },
    #[error("a protocol 5 mode block needs the mode value the server sent")]
    MissingModeValue,
    #[error("{what} was decoded at a different protocol version")]
    VersionMismatch { what: &'static str },
    #[error("packet payload of {size} bytes exceeds the {MAX_PACKET_SIZE}-byte SDK packet limit")]
    PacketTooLarge { size: usize },
}

/// Bounds-checked little-endian reader over a payload.
#[derive(Debug)]
pub struct Reader<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    pub fn new(buf: &'a [u8]) -> Self {
        Self { buf, pos: 0 }
    }

    pub fn remaining(&self) -> usize {
        self.buf.len() - self.pos
    }

    /// Take `n` bytes.
    pub fn bytes(&mut self, n: usize, field: &'static str) -> Result<&'a [u8], DecodeError> {
        let remaining = self.remaining();
        if n > remaining {
            return Err(DecodeError::new(
                field,
                DecodeErrorKind::Truncated {
                    needed: n,
                    remaining,
                },
            ));
        }
        let taken = &self.buf[self.pos..self.pos + n];
        self.pos += n;
        Ok(taken)
    }

    fn array<const N: usize>(&mut self, field: &'static str) -> Result<[u8; N], DecodeError> {
        let mut out = [0u8; N];
        out.copy_from_slice(self.bytes(N, field)?);
        Ok(out)
    }

    pub fn u16(&mut self, field: &'static str) -> Result<u16, DecodeError> {
        self.array(field).map(u16::from_le_bytes)
    }

    pub fn u32(&mut self, field: &'static str) -> Result<u32, DecodeError> {
        self.array(field).map(u32::from_le_bytes)
    }

    pub fn i32(&mut self, field: &'static str) -> Result<i32, DecodeError> {
        self.array(field).map(i32::from_le_bytes)
    }

    /// A string whose `u16` length prefix counts the terminating NUL.
    pub fn string_u16(&mut self, field: &'static str) -> Result<WireString, DecodeError> {
        let len = usize::from(self.u16(field)?);
        self.string_body(len, field)
    }

    /// A string whose `u32` length prefix counts the terminating NUL.
    pub fn string_u32(&mut self, field: &'static str) -> Result<WireString, DecodeError> {
        let len = self.u32(field)? as usize;
        self.string_body(len, field)
    }

    fn string_body(&mut self, len: usize, field: &'static str) -> Result<WireString, DecodeError> {
        let raw = self.bytes(len, field)?;
        match raw.split_last() {
            Some((0, text)) => Ok(WireString::from_bytes(text.to_vec())),
            _ => Err(DecodeError::new(field, DecodeErrorKind::MissingNul)),
        }
    }

    /// `n` little-endian `u32` values, checked against the remaining bytes before
    /// anything is allocated.
    pub fn u32_array(&mut self, n: usize, field: &'static str) -> Result<Vec<u32>, DecodeError> {
        let needed = n.checked_mul(4).ok_or(DecodeError::new(
            field,
            DecodeErrorKind::Truncated {
                needed: usize::MAX,
                remaining: self.remaining(),
            },
        ))?;
        let raw = self.bytes(needed, field)?;
        Ok(raw
            .chunks_exact(4)
            .map(|word| u32::from_le_bytes([word[0], word[1], word[2], word[3]]))
            .collect())
    }

    /// Require that every byte was consumed.
    pub fn finish(self, field: &'static str) -> Result<(), DecodeError> {
        match self.remaining() {
            0 => Ok(()),
            extra => Err(DecodeError::new(
                field,
                DecodeErrorKind::TrailingBytes(extra),
            )),
        }
    }
}

/// Little-endian writer for payloads.
#[derive(Debug, Default)]
pub struct Writer {
    buf: Vec<u8>,
}

impl Writer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.buf.len()
    }

    pub fn is_empty(&self) -> bool {
        self.buf.is_empty()
    }

    pub fn u16(&mut self, value: u16) {
        self.buf.extend_from_slice(&value.to_le_bytes());
    }

    pub fn u32(&mut self, value: u32) {
        self.buf.extend_from_slice(&value.to_le_bytes());
    }

    pub fn i32(&mut self, value: i32) {
        self.buf.extend_from_slice(&value.to_le_bytes());
    }

    /// A count as the `u16` the SDK uses for list lengths.
    pub fn count_u16(&mut self, count: usize, field: &'static str) -> Result<(), EncodeError> {
        let count = u16::try_from(count).map_err(|_| EncodeError::TooManyItems {
            field,
            count,
            max: u64::from(u16::MAX),
        })?;
        self.u16(count);
        Ok(())
    }

    /// A string with a `u16` length prefix that counts the terminating NUL.
    pub fn string_u16(
        &mut self,
        text: &WireString,
        field: &'static str,
    ) -> Result<(), EncodeError> {
        let len =
            u16::try_from(text.as_bytes().len() + 1).map_err(|_| EncodeError::StringTooLong {
                field,
                len: text.as_bytes().len(),
                max: u64::from(u16::MAX) - 1,
            })?;
        self.u16(len);
        self.string_body(text);
        Ok(())
    }

    /// A string with a `u32` length prefix that counts the terminating NUL.
    pub fn string_u32(
        &mut self,
        text: &WireString,
        field: &'static str,
    ) -> Result<(), EncodeError> {
        let len =
            u32::try_from(text.as_bytes().len() + 1).map_err(|_| EncodeError::StringTooLong {
                field,
                len: text.as_bytes().len(),
                max: u64::from(u32::MAX) - 1,
            })?;
        self.u32(len);
        self.string_body(text);
        Ok(())
    }

    fn string_body(&mut self, text: &WireString) {
        self.buf.extend_from_slice(text.as_bytes());
        self.buf.push(0);
    }

    pub fn into_inner(self) -> Vec<u8> {
        self.buf
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn header_golden_bytes() {
        let header = Header {
            dev_id: 0x0102_0304,
            pkt_id: PacketId::UpdateMode,
            size: 0x1234,
        };
        assert_eq!(
            header.encode(),
            [
                b'O', b'R', b'G', b'B', 0x04, 0x03, 0x02, 0x01, 0x4d, 0x04, 0x00, 0x00, 0x34, 0x12,
                0x00, 0x00
            ]
        );
        assert_eq!(Header::decode(&header.encode()), Ok(header));
    }

    #[test]
    fn packet_ids_round_trip() {
        for id in [
            0, 1, 10, 40, 50, 51, 52, 53, 100, 101, 102, 103, 1050, 1101, 1150, 7, 160, 304,
        ] {
            assert_eq!(PacketId::from_wire(id).to_wire(), id);
        }
        assert_eq!(PacketId::from_wire(160), PacketId::Unknown(160));
        assert_eq!(PacketId::from_wire(1101), PacketId::UpdateMode);
    }

    #[test]
    fn string_golden_bytes() {
        let mut writer = Writer::new();
        writer
            .string_u16(&WireString::from("ENE DRAM"), "name")
            .unwrap();
        assert_eq!(
            writer.into_inner(),
            [9, 0, b'E', b'N', b'E', b' ', b'D', b'R', b'A', b'M', 0]
        );

        let mut writer = Writer::new();
        writer
            .string_u32(&WireString::from(""), "configuration")
            .unwrap();
        assert_eq!(writer.into_inner(), [1, 0, 0, 0, 0]);
    }

    #[test]
    fn writer_lengths_and_counts() {
        let mut writer = Writer::new();
        assert!(writer.is_empty());
        writer.u16(1);
        writer.i32(-1);
        assert_eq!(writer.len(), 6);
        assert!(!writer.is_empty());
        assert_eq!(
            writer.count_u16(70_000, "colors"),
            Err(EncodeError::TooManyItems {
                field: "colors",
                count: 70_000,
                max: 65_535
            })
        );
        assert_eq!(writer.into_inner(), [1, 0, 0xff, 0xff, 0xff, 0xff]);
    }

    #[test]
    fn string_decoding_keeps_raw_bytes_and_requires_nul() {
        let mut reader = Reader::new(&[3, 0, 0xff, b'a', 0]);
        let text = reader.string_u16("name").unwrap();
        assert_eq!(text.as_bytes(), [0xff, b'a']);
        reader.finish("name").unwrap();

        let error = Reader::new(&[2, 0, b'a', b'b'])
            .string_u16("name")
            .unwrap_err();
        assert_eq!(error.kind, DecodeErrorKind::MissingNul);

        let error = Reader::new(&[0, 0]).string_u16("name").unwrap_err();
        assert_eq!(error.kind, DecodeErrorKind::MissingNul);

        let error = Reader::new(&[5, 0, b'a', 0])
            .string_u16("name")
            .unwrap_err();
        assert_eq!(
            error.kind,
            DecodeErrorKind::Truncated {
                needed: 5,
                remaining: 2
            }
        );
    }

    #[test]
    fn colour_packing_golden() {
        let colour = RgbColor::from_rgb(0x11, 0x22, 0x33);
        assert_eq!(colour.to_wire(), 0x0033_2211);
        assert_eq!(colour.to_wire().to_le_bytes(), [0x11, 0x22, 0x33, 0x00]);
        assert_eq!(
            (colour.red(), colour.green(), colour.blue()),
            (0x11, 0x22, 0x33)
        );
        assert_eq!(colour.to_string(), "#112233");
        assert_eq!(
            RgbColor::from_wire(0xff33_2211).to_string(),
            "#112233 (top byte 0xff)"
        );
    }

    #[test]
    fn substring_and_equality_ignore_ascii_case() {
        let name = WireString::from("ENE DRAM");
        assert!(name.contains_ignore_ascii_case("ene dram"));
        assert!(name.contains_ignore_ascii_case("DRAM"));
        assert!(!name.contains_ignore_ascii_case("DRAMS"));
        assert!(name.eq_ignore_ascii_case("ene dram"));
        assert!(!name.eq_ignore_ascii_case("ene"));
    }

    fn frame_bytes(dev_id: u32, pkt_id: u32, payload: &[u8]) -> Vec<u8> {
        Frame {
            dev_id,
            pkt_id: PacketId::from_wire(pkt_id),
            payload: payload.to_vec(),
        }
        .encode()
        .unwrap()
    }

    #[test]
    fn framer_reassembles_split_frames() {
        let mut stream = frame_bytes(3, 1, &[1, 2, 3, 4, 5]);
        stream.extend(frame_bytes(0, 100, &[]));
        stream.extend(frame_bytes(9, 160, &[0xaa; 40]));

        for split in 0..stream.len() {
            let mut framer = Framer::new();
            let mut frames = Vec::new();
            for chunk in [&stream[..split], &stream[split..]] {
                framer.push(chunk);
                while let Some(frame) = framer.next_frame().unwrap() {
                    frames.push(frame);
                }
            }
            assert_eq!(frames.len(), 3, "split at {split}");
            assert_eq!(frames[0].payload, [1, 2, 3, 4, 5]);
            assert_eq!(frames[1].pkt_id, PacketId::DeviceListUpdated);
            assert_eq!(frames[2].pkt_id, PacketId::Unknown(160));
            assert_eq!(frames[2].dev_id, 9);
            assert_eq!(framer.buffered(), 0);
        }
    }

    #[test]
    fn framer_rejects_bad_magic_for_good() {
        let mut framer = Framer::new();
        framer.push(b"ORGX\0\0\0\0\0\0\0\0\0\0\0\0");
        assert_eq!(framer.next_frame(), Err(FramingError::BadMagic(*b"ORGX")));
        framer.push(&frame_bytes(0, 100, &[]));
        assert_eq!(framer.next_frame(), Err(FramingError::BadMagic(*b"ORGX")));
        assert_eq!(framer.buffered(), 0);
    }

    #[test]
    fn framer_rejects_oversize_before_buffering_the_payload() {
        let header = Header {
            dev_id: 0,
            pkt_id: PacketId::RequestControllerData,
            size: MAX_PACKET_SIZE + 1,
        };
        let mut framer = Framer::new();
        framer.push(&header.encode());
        assert_eq!(
            framer.next_frame(),
            Err(FramingError::Oversize {
                pkt_id: PacketId::RequestControllerData,
                size: MAX_PACKET_SIZE + 1
            })
        );

        let at_cap = Header {
            size: MAX_PACKET_SIZE,
            ..header
        };
        let mut framer = Framer::new();
        framer.push(&at_cap.encode());
        assert_eq!(framer.next_frame(), Ok(None));
    }

    #[test]
    fn payload_size_is_capped() {
        assert_eq!(payload_size(MAX_PACKET_SIZE as usize), Ok(MAX_PACKET_SIZE));
        assert!(payload_size(MAX_PACKET_SIZE as usize + 1).is_err());
    }

    #[test]
    fn u32_array_checks_length_before_allocating() {
        let error = Reader::new(&[0; 8])
            .u32_array(usize::MAX / 2, "matrix")
            .unwrap_err();
        assert!(matches!(error.kind, DecodeErrorKind::Truncated { .. }));
        assert_eq!(
            Reader::new(&[1, 0, 0, 0, 2, 0, 0, 0])
                .u32_array(2, "m")
                .unwrap(),
            [1, 2]
        );
    }

    proptest! {
        #[test]
        fn framer_never_panics_on_arbitrary_streams(
            chunks in proptest::collection::vec(proptest::collection::vec(any::<u8>(), 0..64), 0..16)
        ) {
            let mut framer = Framer::new();
            for chunk in chunks {
                framer.push(&chunk);
                while let Ok(Some(_)) = framer.next_frame() {}
            }
        }

        #[test]
        fn frames_round_trip_through_the_framer(
            frames in proptest::collection::vec(
                (any::<u32>(), any::<u32>(), proptest::collection::vec(any::<u8>(), 0..128)), 0..8),
            split in 1usize..64
        ) {
            let mut stream = Vec::new();
            for (dev_id, pkt_id, payload) in &frames {
                stream.extend(frame_bytes(*dev_id, *pkt_id, payload));
            }
            let mut framer = Framer::new();
            let mut decoded = Vec::new();
            for chunk in stream.chunks(split) {
                framer.push(chunk);
                while let Some(frame) = framer.next_frame().unwrap() {
                    decoded.push((frame.dev_id, frame.pkt_id.to_wire(), frame.payload));
                }
            }
            prop_assert_eq!(decoded, frames);
        }
    }
}

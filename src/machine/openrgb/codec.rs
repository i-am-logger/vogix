//! Version-gated decoders and encoders for OpenRGB SDK payloads.
//!
//! The block layouts follow `Documentation/OpenRGBSDK.md` ("Device Data",
//! "Mode Data", "Zone Data", "Segment Data", "Matrix Map Data", "LED Data")
//! as the server serialises them (`RGBController::Get*DescriptionData`). Only
//! protocols 5 and 6 are decoded, so the fields gated at protocol 1 (vendor),
//! 3 (brightness), 4 (segments) and 5 (zone and controller flags, LED display
//! names) are always present; the gate that remains is protocol 6, which drops
//! the mode and LED `value` fields and adds display names, device
//! configuration, per-zone modes and segment matrices and flags.
//!
//! Every length and count is checked against the bytes that remain before
//! anything is allocated, and a reply must be consumed exactly: trailing bytes
//! are a decode error, because they mean the block was read at the wrong
//! version.
//!
//! Encoders exist for exactly the packets vogix sends: `REQUEST_CONTROLLER_COUNT`
//! (0), `REQUEST_CONTROLLER_DATA` (1), `REQUEST_PROTOCOL_VERSION` (40),
//! `SET_CLIENT_NAME` (50), `SET_CLIENT_FLAGS` (52), `UPDATELEDS` (1050) and
//! `UPDATEMODE` (1101).

use super::model::{
    AckStatus, ClientFlags, ClientName, ColorMode, ControllerDescription, ControllerFlags,
    ControllerV6, DeviceType, Direction, LedDescription, MatrixMap, ModeDescription, ModeFlags,
    ProtocolVersion, SegmentDescription, SegmentFlags, SegmentV6, ServerFlags, UpdateReason,
    ZoneDescription, ZoneFlags, ZoneType, ZoneV6,
};
use super::wire::{
    DecodeError, DecodeErrorKind, EncodeError, HEADER_LEN, Header, MAX_PACKET_SIZE, PacketId,
    Reader, RgbColor, WireString, Writer, payload_size,
};
use std::collections::BTreeSet;

/// The reply to `REQUEST_CONTROLLER_COUNT`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ControllerList {
    /// Protocol 5: the number of controllers, addressed by index `0..count`.
    Count(u32),
    /// Protocol 6: the unique id of each controller, in server list order.
    Ids(Vec<u32>),
}

/// The body of an `ACK` packet; the header's `dev_id` names the device of the
/// acknowledged packet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AckBody {
    pub acked: PacketId,
    pub status: AckStatus,
}

/// The body of `DETECTION_PROGRESS_CHANGED`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DetectionProgress {
    pub percent: u32,
    pub text: WireString,
}

/// The body of `SIGNALUPDATE`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignalUpdate {
    pub reason: UpdateReason,
    pub body: SignalUpdateBody,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SignalUpdateBody {
    /// `UPDATELEDS` reason: the controller's colours.
    Colours(Vec<RgbColor>),
    /// Every other reason: the whole description.
    Device(Box<ControllerDescription>),
}

fn colours(
    r: &mut Reader<'_>,
    count: u16,
    field: &'static str,
) -> Result<Vec<RgbColor>, DecodeError> {
    Ok(r.u32_array(usize::from(count), field)?
        .into_iter()
        .map(RgbColor::from_wire)
        .collect())
}

/// A `u16` count followed by that many items; the list grows only as items
/// decode, so a large count cannot allocate ahead of the bytes present.
fn list<T>(
    r: &mut Reader<'_>,
    count_field: &'static str,
    mut item: impl FnMut(&mut Reader<'_>) -> Result<T, DecodeError>,
) -> Result<Vec<T>, DecodeError> {
    let count = r.u16(count_field)?;
    let mut items = Vec::new();
    for _ in 0..count {
        items.push(item(r)?);
    }
    Ok(items)
}

/// One Mode Data block.
pub fn decode_mode(
    r: &mut Reader<'_>,
    version: ProtocolVersion,
) -> Result<ModeDescription, DecodeError> {
    let name = r.string_u16("mode_name")?;
    let value = match version {
        ProtocolVersion::V5 => Some(r.i32("mode_value")?),
        ProtocolVersion::V6 => None,
    };
    let flags = ModeFlags::from_bits(r.u32("mode_flags")?);
    let speed_min = r.u32("mode_speed_min")?;
    let speed_max = r.u32("mode_speed_max")?;
    let brightness_min = r.u32("mode_brightness_min")?;
    let brightness_max = r.u32("mode_brightness_max")?;
    let colors_min = r.u32("mode_colors_min")?;
    let colors_max = r.u32("mode_colors_max")?;
    let speed = r.u32("mode_speed")?;
    let brightness = r.u32("mode_brightness")?;
    let direction = Direction(r.u32("mode_direction")?);
    let color_mode = ColorMode::from_wire(r.u32("mode_color_mode")?);
    let count = r.u16("mode_num_colors")?;
    let colors = colours(r, count, "mode_colors")?;
    Ok(ModeDescription {
        name,
        value,
        flags,
        speed_min,
        speed_max,
        brightness_min,
        brightness_max,
        colors_min,
        colors_max,
        speed,
        brightness,
        direction,
        color_mode,
        colors,
    })
}

/// One Mode Data block at `version`. A mode decoded at protocol 5 carries the
/// `value` it must echo back; one decoded at protocol 6 has none.
pub fn encode_mode(
    w: &mut Writer,
    mode: &ModeDescription,
    version: ProtocolVersion,
) -> Result<(), EncodeError> {
    w.string_u16(&mode.name, "mode_name")?;
    match (version, mode.value) {
        (ProtocolVersion::V5, Some(value)) => w.i32(value),
        (ProtocolVersion::V5, None) => return Err(EncodeError::MissingModeValue),
        (ProtocolVersion::V6, Some(_)) => {
            return Err(EncodeError::VersionMismatch { what: "mode block" });
        }
        (ProtocolVersion::V6, None) => {}
    }
    w.u32(mode.flags.bits());
    w.u32(mode.speed_min);
    w.u32(mode.speed_max);
    w.u32(mode.brightness_min);
    w.u32(mode.brightness_max);
    w.u32(mode.colors_min);
    w.u32(mode.colors_max);
    w.u32(mode.speed);
    w.u32(mode.brightness);
    w.u32(mode.direction.0);
    w.u32(mode.color_mode.to_wire());
    w.count_u16(mode.colors.len(), "mode_colors")?;
    for colour in &mode.colors {
        w.u32(colour.to_wire());
    }
    Ok(())
}

/// A `u16` matrix length followed, when it is non-zero, by Matrix Map Data.
/// The length is only a presence marker: the server computes it as a `u16`
/// that wraps for large maps, and its own parser reads `height * width` cells
/// whenever the length is non-zero (`RGBController::SetZoneDescription`).
fn decode_matrix(
    r: &mut Reader<'_>,
    len_field: &'static str,
) -> Result<Option<MatrixMap>, DecodeError> {
    if r.u16(len_field)? == 0 {
        return Ok(None);
    }
    let height = r.u32("matrix_map_height")?;
    let width = r.u32("matrix_map_width")?;
    let cells = usize::try_from(u64::from(height) * u64::from(width))
        .ok()
        .filter(|cells| {
            cells
                .checked_mul(4)
                .is_some_and(|bytes| bytes <= r.remaining())
        })
        .ok_or(DecodeError::new(
            "matrix_map_data",
            DecodeErrorKind::MatrixTooLarge { height, width },
        ))?;
    let map = r.u32_array(cells, "matrix_map_data")?;
    Ok(Some(MatrixMap { height, width, map }))
}

fn decode_segment(
    r: &mut Reader<'_>,
    version: ProtocolVersion,
) -> Result<SegmentDescription, DecodeError> {
    let name = r.string_u16("segment_name")?;
    let segment_type = ZoneType::from_wire(r.u32("segment_type")?);
    let start_idx = r.u32("segment_start_idx")?;
    let leds_count = r.u32("segment_leds_count")?;
    let v6 = match version {
        ProtocolVersion::V5 => None,
        ProtocolVersion::V6 => Some(SegmentV6 {
            matrix: decode_matrix(r, "segment_matrix_len")?,
            flags: SegmentFlags::from_bits(r.u32("segment_flags")?),
        }),
    };
    Ok(SegmentDescription {
        name,
        segment_type,
        start_idx,
        leds_count,
        v6,
    })
}

fn decode_zone(
    r: &mut Reader<'_>,
    version: ProtocolVersion,
) -> Result<ZoneDescription, DecodeError> {
    let name = r.string_u16("zone_name")?;
    let zone_type = ZoneType::from_wire(r.u32("zone_type")?);
    let leds_min = r.u32("zone_leds_min")?;
    let leds_max = r.u32("zone_leds_max")?;
    let leds_count = r.u32("zone_leds_count")?;
    let matrix = decode_matrix(r, "zone_matrix_len")?;
    let segments = list(r, "num_segments", |r| decode_segment(r, version))?;
    let flags = ZoneFlags::from_bits(r.u32("zone_flags")?);
    let v6 = match version {
        ProtocolVersion::V5 => None,
        ProtocolVersion::V6 => {
            let active_mode = r.i32("zone_active_mode")?;
            let modes = list(r, "zone_num_modes", |r| decode_mode(r, version))?;
            let display_name = r.string_u16("zone_display_name")?;
            Some(ZoneV6 {
                active_mode,
                modes,
                display_name,
            })
        }
    };
    Ok(ZoneDescription {
        name,
        zone_type,
        leds_min,
        leds_max,
        leds_count,
        matrix,
        segments,
        flags,
        v6,
    })
}

fn decode_led(r: &mut Reader<'_>, version: ProtocolVersion) -> Result<LedDescription, DecodeError> {
    let name = r.string_u16("led_name")?;
    let value = match version {
        ProtocolVersion::V5 => Some(r.u32("led_value")?),
        ProtocolVersion::V6 => None,
    };
    Ok(LedDescription { name, value })
}

/// One Device Data block.
pub fn decode_device(
    r: &mut Reader<'_>,
    version: ProtocolVersion,
) -> Result<ControllerDescription, DecodeError> {
    let device_type = DeviceType(r.i32("type")?);
    let name = r.string_u16("name")?;
    let vendor = r.string_u16("vendor")?;
    let description = r.string_u16("description")?;
    let firmware_version = r.string_u16("version")?;
    let serial = r.string_u16("serial")?;
    let location = r.string_u16("location")?;
    let num_modes = r.u16("num_modes")?;
    let active_mode = r.i32("active_mode")?;
    let mut modes = Vec::new();
    for _ in 0..num_modes {
        modes.push(decode_mode(r, version)?);
    }
    let zones = list(r, "num_zones", |r| decode_zone(r, version))?;
    let leds = list(r, "num_leds", |r| decode_led(r, version))?;
    let num_colors = r.u16("num_colors")?;
    let colors = colours(r, num_colors, "colors")?;
    let led_display_names = list(r, "num_led_display_names", |r| {
        r.string_u16("led_display_name")
    })?;
    let flags = ControllerFlags::from_bits(r.u32("flags")?);
    let v6 = match version {
        ProtocolVersion::V5 => None,
        ProtocolVersion::V6 => Some(ControllerV6 {
            display_name: r.string_u16("display_name")?,
            configuration: r.string_u32("configuration")?,
        }),
    };
    Ok(ControllerDescription {
        device_type,
        name,
        vendor,
        description,
        version: firmware_version,
        serial,
        location,
        active_mode,
        modes,
        zones,
        leds,
        colors,
        led_display_names,
        flags,
        v6,
    })
}

/// A leading `u32 data_size` that must equal the payload length, as the server
/// writes for controller data, `SIGNALUPDATE` and detection progress.
fn data_size_prefix(r: &mut Reader<'_>, payload_len: usize) -> Result<(), DecodeError> {
    let declared = r.u32("data_size")?;
    if declared as usize != payload_len {
        return Err(DecodeError::new(
            "data_size",
            DecodeErrorKind::SizeMismatch {
                declared,
                actual: payload_len,
            },
        ));
    }
    Ok(())
}

/// The `REQUEST_CONTROLLER_DATA` reply: `data_size` then one Device Data block.
pub fn decode_controller_data(
    payload: &[u8],
    version: ProtocolVersion,
) -> Result<ControllerDescription, DecodeError> {
    let mut r = Reader::new(payload);
    data_size_prefix(&mut r, payload.len())?;
    let description = decode_device(&mut r, version)?;
    r.finish("controller data")?;
    Ok(description)
}

/// The `REQUEST_CONTROLLER_COUNT` reply.
pub fn decode_controller_count(
    payload: &[u8],
    version: ProtocolVersion,
) -> Result<ControllerList, DecodeError> {
    let mut r = Reader::new(payload);
    let count = r.u32("controller_count")?;
    let list = match version {
        ProtocolVersion::V5 => ControllerList::Count(count),
        ProtocolVersion::V6 => {
            let ids = r.u32_array(count as usize, "controller_ids")?;
            let mut seen = BTreeSet::new();
            if let Some(duplicate) = ids.iter().find(|id| !seen.insert(**id)) {
                return Err(DecodeError::new(
                    "controller_ids",
                    DecodeErrorKind::DuplicateId(*duplicate),
                ));
            }
            ControllerList::Ids(ids)
        }
    };
    r.finish("controller count")?;
    Ok(list)
}

/// The `REQUEST_PROTOCOL_VERSION` reply: the server's highest version.
pub fn decode_protocol_version(payload: &[u8]) -> Result<u32, DecodeError> {
    let mut r = Reader::new(payload);
    let version = r.u32("protocol_version")?;
    r.finish("protocol version")?;
    Ok(version)
}

/// An `ACK` body.
pub fn decode_ack(payload: &[u8]) -> Result<AckBody, DecodeError> {
    let mut r = Reader::new(payload);
    let acked = PacketId::from_wire(r.u32("acked_pkt_id")?);
    let status = AckStatus::from_wire(r.u32("status")?);
    r.finish("ack")?;
    Ok(AckBody { acked, status })
}

/// A `SET_SERVER_NAME` body: a NUL-terminated string filling the payload.
pub fn decode_server_name(payload: &[u8]) -> Result<WireString, DecodeError> {
    match payload.split_last() {
        Some((0, text)) => Ok(WireString::from_bytes(text.to_vec())),
        _ => Err(DecodeError::new("server_name", DecodeErrorKind::MissingNul)),
    }
}

/// A `SET_SERVER_FLAGS` body.
pub fn decode_server_flags(payload: &[u8]) -> Result<ServerFlags, DecodeError> {
    let mut r = Reader::new(payload);
    let flags = ServerFlags::from_bits(r.u32("server_flags")?);
    r.finish("server flags")?;
    Ok(flags)
}

/// A `DETECTION_PROGRESS_CHANGED` body.
pub fn decode_detection_progress(payload: &[u8]) -> Result<DetectionProgress, DecodeError> {
    let mut r = Reader::new(payload);
    data_size_prefix(&mut r, payload.len())?;
    let percent = r.u32("detection_percent")?;
    let text = r.string_u16("detection_string")?;
    r.finish("detection progress")?;
    Ok(DetectionProgress { percent, text })
}

/// A `SIGNALUPDATE` body: `data_size`, the update reason, then either the
/// colours (`UPDATELEDS`) or a whole Device Data block.
pub fn decode_signal_update(
    payload: &[u8],
    version: ProtocolVersion,
) -> Result<SignalUpdate, DecodeError> {
    let mut r = Reader::new(payload);
    data_size_prefix(&mut r, payload.len())?;
    let reason = UpdateReason::from_wire(r.u32("update_reason")?);
    let body = match reason {
        UpdateReason::UpdateLeds => {
            let count = r.u16("num_colors")?;
            SignalUpdateBody::Colours(colours(&mut r, count, "colors")?)
        }
        _ => SignalUpdateBody::Device(Box::new(decode_device(&mut r, version)?)),
    };
    r.finish("signal update")?;
    Ok(SignalUpdate { reason, body })
}

/// Header plus a payload whose size the caller has already bounded below
/// [`MAX_PACKET_SIZE`].
fn packet(dev_id: u32, pkt_id: PacketId, size: u32, payload: &[u8]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(HEADER_LEN + payload.len());
    bytes.extend_from_slice(
        &Header {
            dev_id,
            pkt_id,
            size,
        }
        .encode(),
    );
    bytes.extend_from_slice(payload);
    bytes
}

/// `REQUEST_PROTOCOL_VERSION` carrying the client's highest version.
pub fn request_protocol_version(max: ProtocolVersion) -> Vec<u8> {
    packet(
        0,
        PacketId::RequestProtocolVersion,
        4,
        &max.number().to_le_bytes(),
    )
}

/// `REQUEST_CONTROLLER_COUNT`.
pub fn request_controller_count() -> Vec<u8> {
    packet(0, PacketId::RequestControllerCount, 0, &[])
}

/// `REQUEST_CONTROLLER_DATA` for `dev_id` (an index at protocol 5, a unique id
/// at protocol 6), carrying the negotiated version the reply is serialised at.
pub fn request_controller_data(dev_id: u32, version: ProtocolVersion) -> Vec<u8> {
    packet(
        dev_id,
        PacketId::RequestControllerData,
        4,
        &version.number().to_le_bytes(),
    )
}

/// `SET_CLIENT_NAME`: the NUL-terminated name.
pub fn set_client_name(name: &ClientName) -> Vec<u8> {
    let mut payload = name.as_str().as_bytes().to_vec();
    payload.push(0);
    // ClientName::new bounds the name below MAX_PACKET_SIZE bytes.
    let size = u32::try_from(payload.len()).unwrap_or(MAX_PACKET_SIZE);
    packet(0, PacketId::SetClientName, size, &payload)
}

/// `SET_CLIENT_FLAGS`.
pub fn set_client_flags(flags: ClientFlags) -> Vec<u8> {
    packet(0, PacketId::SetClientFlags, 4, &flags.bits().to_le_bytes())
}

/// A write whose payload starts with `u32 data_size` equal to the packet size,
/// which the server checks before applying it
/// (`ProcessRequest_RGBController_UpdateLEDs` / `_UpdateSaveMode`).
fn sized_write(dev_id: u32, pkt_id: PacketId, body: Writer) -> Result<Vec<u8>, EncodeError> {
    let size = payload_size(4 + body.len())?;
    let mut payload = Vec::with_capacity(size as usize);
    payload.extend_from_slice(&size.to_le_bytes());
    payload.extend_from_slice(&body.into_inner());
    Ok(packet(dev_id, pkt_id, size, &payload))
}

/// `UPDATELEDS`: one colour per LED of the controller.
pub fn update_leds(dev_id: u32, colours: &[RgbColor]) -> Result<Vec<u8>, EncodeError> {
    let mut body = Writer::new();
    body.count_u16(colours.len(), "led_color")?;
    for colour in colours {
        body.u32(colour.to_wire());
    }
    sized_write(dev_id, PacketId::UpdateLeds, body)
}

/// `UPDATEMODE`: the mode index and the mode block to apply at it.
pub fn update_mode(
    dev_id: u32,
    mode_index: usize,
    mode: &ModeDescription,
    version: ProtocolVersion,
) -> Result<Vec<u8>, EncodeError> {
    let index = i32::try_from(mode_index).map_err(|_| EncodeError::TooManyItems {
        field: "mode_idx",
        count: mode_index,
        max: i32::MAX as u64,
    })?;
    let mut body = Writer::new();
    body.i32(index);
    encode_mode(&mut body, mode, version)?;
    sized_write(dev_id, PacketId::UpdateMode, body)
}

#[cfg(test)]
mod tests {
    use super::super::testkit::{self, alloc_probe};
    use super::*;
    use proptest::prelude::*;

    const VERSIONS: [ProtocolVersion; 2] = [ProtocolVersion::V5, ProtocolVersion::V6];

    fn mode_bytes(mode: &ModeDescription, version: ProtocolVersion) -> Vec<u8> {
        let mut w = Writer::new();
        encode_mode(&mut w, mode, version).unwrap();
        w.into_inner()
    }

    fn header_of(bytes: &[u8]) -> Header {
        let mut raw = [0u8; HEADER_LEN];
        raw.copy_from_slice(&bytes[..HEADER_LEN]);
        Header::decode(&raw).unwrap()
    }

    #[test]
    fn request_golden_bytes() {
        assert_eq!(
            request_protocol_version(ProtocolVersion::V6),
            [
                b'O', b'R', b'G', b'B', 0, 0, 0, 0, 40, 0, 0, 0, 4, 0, 0, 0, 6, 0, 0, 0
            ]
        );
        assert_eq!(
            request_controller_count(),
            [b'O', b'R', b'G', b'B', 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
        );
        assert_eq!(
            request_controller_data(7, ProtocolVersion::V5),
            [
                b'O', b'R', b'G', b'B', 7, 0, 0, 0, 1, 0, 0, 0, 4, 0, 0, 0, 5, 0, 0, 0
            ]
        );
        assert_eq!(
            set_client_name(&ClientName::new("vogix").unwrap()),
            [
                b'O', b'R', b'G', b'B', 0, 0, 0, 0, 50, 0, 0, 0, 6, 0, 0, 0, b'v', b'o', b'g',
                b'i', b'x', 0
            ]
        );
        assert_eq!(
            set_client_flags(ClientFlags::SUPPORTS_RGBCONTROLLER),
            [
                b'O', b'R', b'G', b'B', 0, 0, 0, 0, 52, 0, 0, 0, 4, 0, 0, 0, 1, 0, 0, 0
            ]
        );
    }

    #[test]
    fn update_leds_golden_bytes_and_data_size_equals_packet_size() {
        let bytes = update_leds(
            3,
            &[RgbColor::from_rgb(1, 2, 3), RgbColor::from_rgb(4, 5, 6)],
        )
        .unwrap();
        assert_eq!(
            bytes,
            [
                b'O', b'R', b'G', b'B', 3, 0, 0, 0, 0x1a, 0x04, 0, 0, 14, 0, 0, 0, // header
                14, 0, 0, 0, // data_size
                2, 0, // num_colors
                1, 2, 3, 0, 4, 5, 6, 0,
            ]
        );
    }

    #[test]
    fn update_mode_carries_index_and_block_with_data_size_equal_to_packet_size() {
        for version in VERSIONS {
            let mode = testkit::static_per_led_mode(version);
            let bytes = update_mode(9, 2, &mode, version).unwrap();
            let header = header_of(&bytes);
            assert_eq!(header.pkt_id, PacketId::UpdateMode);
            assert_eq!(header.dev_id, 9);
            let payload = &bytes[HEADER_LEN..];
            assert_eq!(header.size as usize, payload.len());
            assert_eq!(
                u32::from_le_bytes(payload[0..4].try_into().unwrap()),
                header.size
            );
            assert_eq!(i32::from_le_bytes(payload[4..8].try_into().unwrap()), 2);
            let mut r = Reader::new(&payload[8..]);
            assert_eq!(decode_mode(&mut r, version).unwrap(), mode);
            r.finish("mode").unwrap();
        }
    }

    #[test]
    fn mode_encoding_refuses_a_block_from_the_other_version() {
        let v5 = testkit::static_per_led_mode(ProtocolVersion::V5);
        let mut w = Writer::new();
        assert_eq!(
            encode_mode(&mut w, &v5, ProtocolVersion::V6),
            Err(EncodeError::VersionMismatch { what: "mode block" })
        );
        let v6 = testkit::static_per_led_mode(ProtocolVersion::V6);
        assert_eq!(
            encode_mode(&mut Writer::new(), &v6, ProtocolVersion::V5),
            Err(EncodeError::MissingModeValue)
        );
    }

    #[test]
    fn mode_block_golden_bytes_at_both_versions() {
        let mut mode = testkit::static_per_led_mode(ProtocolVersion::V5);
        mode.value = Some(1);
        mode.colors = vec![RgbColor::from_rgb(0xaa, 0xbb, 0xcc)];
        let expected_v5: Vec<u8> = [
            &[7u8, 0][..],
            b"Static\0",
            &1i32.to_le_bytes(),    // value (protocol 5)
            &0x20u32.to_le_bytes(), // flags: HAS_PER_LED_COLOR
            &[0; 4 * 8],            // speed_min .. brightness
            &0u32.to_le_bytes(),    // direction
            &1u32.to_le_bytes(),    // color_mode: per-LED
            &1u16.to_le_bytes(),    // num_colors
            &[0xaa, 0xbb, 0xcc, 0x00],
        ]
        .concat();
        assert_eq!(mode_bytes(&mode, ProtocolVersion::V5), expected_v5);

        mode.value = None;
        let expected_v6: Vec<u8> = [&expected_v5[..9], &expected_v5[13..]].concat();
        assert_eq!(mode_bytes(&mode, ProtocolVersion::V6), expected_v6);
    }

    #[test]
    fn controller_data_round_trips_at_both_versions() {
        for version in VERSIONS {
            for description in [
                testkit::ene_dram(version, 0),
                testkit::govee(version),
                testkit::keyboard_with_matrix_and_segments(version),
            ] {
                let payload = testkit::controller_data_payload(&description, version);
                assert_eq!(
                    decode_controller_data(&payload, version).unwrap(),
                    description
                );
            }
        }
    }

    #[test]
    fn controller_data_rejects_the_other_versions_layout_and_bad_sizes() {
        let v6 = testkit::controller_data_payload(
            &testkit::ene_dram(ProtocolVersion::V6, 0),
            ProtocolVersion::V6,
        );
        assert!(decode_controller_data(&v6, ProtocolVersion::V5).is_err());
        let v5 = testkit::controller_data_payload(
            &testkit::ene_dram(ProtocolVersion::V5, 0),
            ProtocolVersion::V5,
        );
        assert!(decode_controller_data(&v5, ProtocolVersion::V6).is_err());

        let mut wrong_size = v6.clone();
        wrong_size[0] ^= 1;
        assert!(matches!(
            decode_controller_data(&wrong_size, ProtocolVersion::V6)
                .unwrap_err()
                .kind,
            DecodeErrorKind::SizeMismatch { .. }
        ));

        let mut trailing = v6;
        trailing.push(0);
        let size = trailing.len() as u32;
        trailing[0..4].copy_from_slice(&size.to_le_bytes());
        assert_eq!(
            decode_controller_data(&trailing, ProtocolVersion::V6)
                .unwrap_err()
                .kind,
            DecodeErrorKind::TrailingBytes(1)
        );
    }

    #[test]
    fn controller_count_at_both_versions() {
        assert_eq!(
            decode_controller_count(&3u32.to_le_bytes(), ProtocolVersion::V5),
            Ok(ControllerList::Count(3))
        );
        let v6: Vec<u8> = [2u32, 17, 4].iter().flat_map(|w| w.to_le_bytes()).collect();
        assert_eq!(
            decode_controller_count(&v6, ProtocolVersion::V6),
            Ok(ControllerList::Ids(vec![17, 4]))
        );
        let short: Vec<u8> = [3u32, 17, 4].iter().flat_map(|w| w.to_le_bytes()).collect();
        assert!(decode_controller_count(&short, ProtocolVersion::V6).is_err());
        let duplicate: Vec<u8> = [2u32, 4, 4].iter().flat_map(|w| w.to_le_bytes()).collect();
        assert_eq!(
            decode_controller_count(&duplicate, ProtocolVersion::V6)
                .unwrap_err()
                .kind,
            DecodeErrorKind::DuplicateId(4)
        );
        assert!(decode_controller_count(&v6, ProtocolVersion::V5).is_err());
        let huge = u32::MAX.to_le_bytes();
        assert!(decode_controller_count(&huge, ProtocolVersion::V6).is_err());
    }

    #[test]
    fn small_replies() {
        assert_eq!(decode_protocol_version(&6u32.to_le_bytes()), Ok(6));
        assert!(decode_protocol_version(&[6, 0, 0]).is_err());
        let ack: Vec<u8> = [1101u32, 4].iter().flat_map(|w| w.to_le_bytes()).collect();
        assert_eq!(
            decode_ack(&ack),
            Ok(AckBody {
                acked: PacketId::UpdateMode,
                status: AckStatus::ErrorInvalidId
            })
        );
        assert_eq!(
            decode_server_name(b"OpenRGB\0").unwrap(),
            WireString::from("OpenRGB")
        );
        assert!(decode_server_name(b"OpenRGB").is_err());
        assert!(decode_server_name(b"").is_err());
        assert_eq!(
            decode_server_flags(&0x61u32.to_le_bytes()),
            Ok(ServerFlags::from_bits(0x61))
        );
        let progress: Vec<u8> = [
            &17u32.to_le_bytes()[..],
            &40u32.to_le_bytes(),
            &[7, 0],
            b"ENE Bu\0",
        ]
        .concat();
        assert_eq!(
            decode_detection_progress(&progress),
            Ok(DetectionProgress {
                percent: 40,
                text: WireString::from("ENE Bu")
            })
        );
    }

    #[test]
    fn signal_update_bodies() {
        let colours = [RgbColor::from_rgb(1, 2, 3)];
        let leds = testkit::signal_update_leds_payload(&colours);
        assert_eq!(
            decode_signal_update(&leds, ProtocolVersion::V6),
            Ok(SignalUpdate {
                reason: UpdateReason::UpdateLeds,
                body: SignalUpdateBody::Colours(colours.to_vec())
            })
        );
        let description = testkit::ene_dram(ProtocolVersion::V6, 1);
        let device = testkit::signal_update_device_payload(
            UpdateReason::UpdateMode,
            &description,
            ProtocolVersion::V6,
        );
        assert_eq!(
            decode_signal_update(&device, ProtocolVersion::V6),
            Ok(SignalUpdate {
                reason: UpdateReason::UpdateMode,
                body: SignalUpdateBody::Device(Box::new(description))
            })
        );
    }

    #[test]
    fn the_allocation_probe_sees_allocations_on_this_thread_only() {
        // black_box keeps release builds from eliding the allocations.
        let (buffer, peak) =
            alloc_probe::peak_allocation(|| std::hint::black_box(vec![0u8; 100_000]));
        assert_eq!(buffer.len(), 100_000);
        assert!(peak >= 100_000, "peak {peak}");
        let (_, idle) = alloc_probe::peak_allocation(|| std::hint::black_box(1) + 1);
        assert_eq!(idle, 0);
        let (_, other_thread) = alloc_probe::peak_allocation(|| {
            std::thread::spawn(|| std::hint::black_box(vec![0u8; 100_000]).len())
                .join()
                .unwrap()
        });
        assert!(other_thread < 100_000, "peak {other_thread}");
    }

    #[test]
    fn matrix_larger_than_the_packet_is_rejected_without_allocating() {
        let mut description = testkit::keyboard_with_matrix_and_segments(ProtocolVersion::V6);
        description.zones[0].matrix = Some(MatrixMap {
            height: 1,
            width: 1,
            map: vec![0xc0ff_ee11],
        });
        let mut payload = testkit::controller_data_payload(&description, ProtocolVersion::V6);
        // Rewrite the zone matrix to claim 65535 x 65535 cells.
        let marker = [
            1u32.to_le_bytes(),
            1u32.to_le_bytes(),
            0xc0ff_ee11u32.to_le_bytes(),
        ]
        .concat();
        let at = payload
            .windows(marker.len())
            .position(|w| w == marker.as_slice())
            .unwrap();
        payload[at..at + 4].copy_from_slice(&0xffffu32.to_le_bytes());
        payload[at + 4..at + 8].copy_from_slice(&0xffffu32.to_le_bytes());
        let (result, peak) =
            alloc_probe::peak_allocation(|| decode_controller_data(&payload, ProtocolVersion::V6));
        assert_eq!(
            result.unwrap_err().kind,
            DecodeErrorKind::MatrixTooLarge {
                height: 0xffff,
                width: 0xffff
            }
        );
        assert!(peak < 64 * 1024, "peak allocation {peak}");
    }

    /// Every decoder over `bytes`, at every version.
    fn decode_everything(bytes: &[u8]) {
        for version in VERSIONS {
            let _ = decode_controller_data(bytes, version);
            let _ = decode_device(&mut Reader::new(bytes), version);
            let _ = decode_mode(&mut Reader::new(bytes), version);
            let _ = decode_zone(&mut Reader::new(bytes), version);
            let _ = decode_segment(&mut Reader::new(bytes), version);
            let _ = decode_led(&mut Reader::new(bytes), version);
            let _ = decode_controller_count(bytes, version);
            let _ = decode_signal_update(bytes, version);
        }
        let _ = decode_matrix(&mut Reader::new(bytes), "matrix");
        let _ = decode_protocol_version(bytes);
        let _ = decode_ack(bytes);
        let _ = decode_server_name(bytes);
        let _ = decode_server_flags(bytes);
        let _ = decode_detection_progress(bytes);
    }

    /// A bound on what decoding `len` input bytes may allocate at once: the
    /// decoded structures are larger than their encoded form, and `Vec` doubles
    /// its capacity, but nothing may scale with a count the bytes do not back.
    fn allocation_bound(len: usize) -> usize {
        64 * len + 4096
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(512))]

        #[test]
        fn mode_blocks_round_trip(version in prop_oneof![Just(ProtocolVersion::V5), Just(ProtocolVersion::V6)],
                                  seed in any::<u64>()) {
            let mode = testkit::arbitrary_mode(version, seed);
            let bytes = mode_bytes(&mode, version);
            let mut r = Reader::new(&bytes);
            let decoded = decode_mode(&mut r, version).unwrap();
            r.finish("mode").unwrap();
            prop_assert_eq!(&decoded, &mode);
            prop_assert_eq!(mode_bytes(&decoded, version), bytes);
        }

        #[test]
        fn mode_blocks_from_arbitrary_valid_bytes_re_encode_exactly(
            version in prop_oneof![Just(ProtocolVersion::V5), Just(ProtocolVersion::V6)],
            name in proptest::collection::vec(1u8..=255, 0..24),
            words in proptest::array::uniform12(any::<u32>()),
            colours in proptest::collection::vec(any::<u32>(), 0..8),
        ) {
            let mut bytes = Vec::new();
            bytes.extend_from_slice(&(name.len() as u16 + 1).to_le_bytes());
            bytes.extend_from_slice(&name);
            bytes.push(0);
            let fields = match version { ProtocolVersion::V5 => 12, ProtocolVersion::V6 => 11 };
            for word in &words[..fields] {
                bytes.extend_from_slice(&word.to_le_bytes());
            }
            bytes.extend_from_slice(&(colours.len() as u16).to_le_bytes());
            for colour in &colours {
                bytes.extend_from_slice(&colour.to_le_bytes());
            }
            let mut r = Reader::new(&bytes);
            let decoded = decode_mode(&mut r, version).unwrap();
            r.finish("mode").unwrap();
            prop_assert_eq!(mode_bytes(&decoded, version), bytes);
        }

        #[test]
        fn controllers_round_trip(version in prop_oneof![Just(ProtocolVersion::V5), Just(ProtocolVersion::V6)],
                                  seed in any::<u64>()) {
            let description = testkit::arbitrary_controller(version, seed);
            let payload = testkit::controller_data_payload(&description, version);
            prop_assert_eq!(decode_controller_data(&payload, version).unwrap(), description);
        }

        #[test]
        fn random_bytes_never_panic_or_over_allocate(bytes in proptest::collection::vec(any::<u8>(), 0..512)) {
            let ((), peak) = alloc_probe::peak_allocation(|| decode_everything(&bytes));
            prop_assert!(peak <= allocation_bound(bytes.len()), "peak {} for {} bytes", peak, bytes.len());
        }

        #[test]
        fn truncated_valid_payloads_are_errors(version in prop_oneof![Just(ProtocolVersion::V5), Just(ProtocolVersion::V6)],
                                               seed in any::<u64>(), cut in any::<prop::sample::Index>()) {
            let description = testkit::arbitrary_controller(version, seed);
            let payload = testkit::controller_data_payload(&description, version);
            let cut = cut.index(payload.len());
            let truncated = &payload[..cut];
            let (result, peak) = alloc_probe::peak_allocation(|| decode_controller_data(truncated, version));
            prop_assert!(result.is_err());
            prop_assert!(peak <= allocation_bound(truncated.len()));
            // The body without its data_size prefix, cut short, is an error too.
            let body = &payload[4..];
            let body_cut = &body[..cut.min(body.len()).saturating_sub(1)];
            prop_assert!(decode_device(&mut Reader::new(body_cut), version).is_err());
            decode_everything(truncated);
        }

        #[test]
        fn corrupted_valid_payloads_never_panic(version in prop_oneof![Just(ProtocolVersion::V5), Just(ProtocolVersion::V6)],
                                                seed in any::<u64>(),
                                                flips in proptest::collection::vec((any::<prop::sample::Index>(), any::<u8>()), 1..8)) {
            let description = testkit::arbitrary_controller(version, seed);
            let mut payload = testkit::controller_data_payload(&description, version);
            for (at, byte) in flips {
                let at = at.index(payload.len());
                payload[at] = byte;
            }
            let ((), peak) = alloc_probe::peak_allocation(|| decode_everything(&payload));
            prop_assert!(peak <= allocation_bound(payload.len()));
        }
    }
}

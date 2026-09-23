//! Tests on controller payloads captured from the real OpenRGB server.
//!
//! `tests/fixtures/openrgb/v6` and `v5` hold what `vogix machine inspect
//! --capture` wrote in the `openrgb-owner` VM check: vogix's OpenRGB build
//! serving a DDP device ("ENE DRAM Wire"), two Debug DRAM sticks ("ENE DRAM"),
//! a Debug keyboard with key and underglow matrix zones, and a Govee device,
//! after the owner applied the palette nordic — base01 #3b4252 to every
//! "ENE DRAM" controller (Direct, per-LED) and base0D #81a1c1 to the Govee
//! device (Static, mode-specific). Each directory holds every controller's
//! `REQUEST_CONTROLLER_DATA` payload at that protocol and the capture's
//! manifest.

use super::codec;
use super::inspect::CaptureManifest;
use super::model::{
    ColorMode, ControllerDescription, ControllerFlags, DeviceType, Direction, ModeDescription,
    ModeFlags, ProtocolVersion, ZoneFlags, ZoneType,
};
use super::select::{self, ApplyKind, ColourPath, PlanError, Selector, Writes};
use super::testkit;
use super::wire::{Reader, RgbColor, WireString, Writer};

const BASE01: RgbColor = RgbColor::from_rgb(0x3b, 0x42, 0x52);
const BASE0D: RgbColor = RgbColor::from_rgb(0x81, 0xa1, 0xc1);
const BLACK: RgbColor = RgbColor::from_rgb(0, 0, 0);
/// The matrix-map value of a cell with no LED.
const NO_LED: u32 = 0xffff_ffff;

struct Capture {
    protocol: ProtocolVersion,
    manifest: CaptureManifest,
    payloads: [&'static [u8]; 5],
}

macro_rules! fixture {
    ($dir:literal, $file:literal) => {
        concat!("../../../tests/fixtures/openrgb/", $dir, "/", $file)
    };
}

macro_rules! capture {
    ($dir:literal) => {
        (
            include_str!(fixture!($dir, "manifest.json")),
            [
                include_bytes!(fixture!($dir, "controller-00.bin")).as_slice(),
                include_bytes!(fixture!($dir, "controller-01.bin")).as_slice(),
                include_bytes!(fixture!($dir, "controller-02.bin")).as_slice(),
                include_bytes!(fixture!($dir, "controller-03.bin")).as_slice(),
                include_bytes!(fixture!($dir, "controller-04.bin")).as_slice(),
            ],
        )
    };
}

const PROTOCOLS: [ProtocolVersion; 2] = [ProtocolVersion::V5, ProtocolVersion::V6];

fn load(protocol: ProtocolVersion) -> Capture {
    let (manifest, payloads) = match protocol {
        ProtocolVersion::V5 => capture!("v5"),
        ProtocolVersion::V6 => capture!("v6"),
    };
    Capture {
        protocol,
        manifest: serde_json::from_str(manifest).expect("the capture manifest parses"),
        payloads,
    }
}

impl Capture {
    /// Every captured payload with its display name, in server list order.
    fn payloads(&self) -> impl Iterator<Item = (&str, &'static [u8])> {
        self.manifest
            .controllers
            .iter()
            .zip(self.payloads)
            .map(|(entry, raw)| (entry.display_name.as_str(), raw))
    }

    fn decode(&self, raw: &[u8]) -> ControllerDescription {
        codec::decode_controller_data(raw, self.protocol)
            .unwrap_or_else(|e| panic!("protocol {} payload: {e}", self.protocol))
    }

    /// The decoded controllers with this display name.
    fn named(&self, name: &str) -> Vec<ControllerDescription> {
        self.payloads()
            .filter(|(display_name, _)| *display_name == name)
            .map(|(_, raw)| self.decode(raw))
            .collect()
    }

    fn one(&self, name: &str) -> ControllerDescription {
        let mut found = self.named(name);
        assert_eq!(found.len(), 1, "{name}");
        found.remove(0)
    }
}

fn names(items: impl IntoIterator<Item = WireString>) -> Vec<String> {
    items.into_iter().map(|s| s.to_string()).collect()
}

/// The byte range of each device-level mode block in a controller payload.
fn mode_blocks(raw: &[u8], protocol: ProtocolVersion) -> Vec<(ModeDescription, &[u8])> {
    let mut r = Reader::new(raw);
    r.u32("data_size").unwrap();
    r.i32("type").unwrap();
    for field in [
        "name",
        "vendor",
        "description",
        "version",
        "serial",
        "location",
    ] {
        r.string_u16(field).unwrap();
    }
    let count = r.u16("num_modes").unwrap();
    r.i32("active_mode").unwrap();
    (0..count)
        .map(|_| {
            let start = raw.len() - r.remaining();
            let mode = codec::decode_mode(&mut r, protocol).unwrap();
            let end = raw.len() - r.remaining();
            (mode, &raw[start..end])
        })
        .collect()
}

#[test]
fn the_manifests_describe_the_captured_payloads() {
    for protocol in PROTOCOLS {
        let capture = load(protocol);
        let manifest = &capture.manifest;
        assert_eq!(manifest.protocol, protocol);
        assert_eq!(
            manifest.server_max, 6,
            "vogix's OpenRGB build speaks protocol 6"
        );
        // SET_SERVER_NAME exists from protocol 6 on.
        let server_name = match protocol {
            ProtocolVersion::V5 => None,
            ProtocolVersion::V6 => Some("OpenRGB 1.0+ (git)"),
        };
        assert_eq!(manifest.server_name.as_deref(), server_name);
        let mut listed: Vec<&str> = capture.payloads().map(|(name, _)| name).collect();
        listed.sort_unstable();
        assert_eq!(
            listed,
            [
                "Debug Keyboard",
                "ENE DRAM",
                "ENE DRAM",
                "ENE DRAM Wire",
                "Govee "
            ]
        );
        for (index, (entry, raw)) in manifest
            .controllers
            .iter()
            .zip(capture.payloads)
            .enumerate()
        {
            assert_eq!(entry.index, index);
            assert_eq!(entry.file, format!("controller-{index:02}.bin"));
            assert_eq!(entry.bytes, raw.len());
            if protocol == ProtocolVersion::V5 {
                assert_eq!(
                    entry.dev_id as usize, index,
                    "protocol 5 addresses are indices"
                );
            }
        }
    }
}

#[test]
fn every_captured_payload_decodes_and_reencodes_byte_for_byte() {
    for protocol in PROTOCOLS {
        let capture = load(protocol);
        for (name, raw) in capture.payloads() {
            let description = capture.decode(raw);
            assert_eq!(description.display_name().to_string(), name);
            assert_eq!(
                testkit::controller_data_payload(&description, protocol),
                raw,
                "{name} at protocol {protocol}"
            );
        }
    }
}

#[test]
fn every_captured_mode_block_reencodes_byte_for_byte_with_the_updatemode_encoder() {
    let mut blocks = 0;
    for protocol in PROTOCOLS {
        let capture = load(protocol);
        for (name, raw) in capture.payloads() {
            let description = capture.decode(raw);
            let found = mode_blocks(raw, protocol);
            assert_eq!(found.len(), description.modes.len(), "{name}");
            for ((mode, bytes), decoded) in found.iter().zip(&description.modes) {
                assert_eq!(mode, decoded);
                let mut w = Writer::new();
                codec::encode_mode(&mut w, mode, protocol).unwrap();
                assert_eq!(
                    w.into_inner(),
                    *bytes,
                    "{name} mode {} at {protocol}",
                    mode.name
                );
                blocks += 1;
            }
        }
    }
    // Direct on the DDP device, both DRAM sticks and the keyboard, and
    // Static and Direct on the Govee device, at both protocols.
    assert_eq!(blocks, 12);
}

#[test]
fn a_payload_does_not_decode_at_the_other_protocol() {
    for (protocol, other) in [
        (ProtocolVersion::V5, ProtocolVersion::V6),
        (ProtocolVersion::V6, ProtocolVersion::V5),
    ] {
        let capture = load(protocol);
        for (name, raw) in capture.payloads() {
            assert!(
                codec::decode_controller_data(raw, other).is_err(),
                "{name} captured at {protocol} decoded at {other}"
            );
        }
    }
}

#[test]
fn the_debug_dram_sticks_decode_exactly() {
    for protocol in PROTOCOLS {
        let capture = load(protocol);
        let sticks = capture.named("ENE DRAM");
        assert_eq!(sticks.len(), 2);
        assert_eq!(sticks[0], sticks[1], "two identically declared sticks");
        let dram = &sticks[0];

        assert_eq!(dram.device_type, DeviceType(1));
        assert_eq!(dram.device_type.name(), Some("dram"));
        assert_eq!(dram.name.to_string(), "ENE DRAM");
        assert_eq!(dram.vendor.to_string(), "Debug DRAM Vendor String");
        assert_eq!(dram.description.to_string(), "Debug DRAM Device");
        assert_eq!(dram.version.to_string(), "Debug DRAM Version String");
        assert_eq!(dram.serial.to_string(), "Debug DRAM Serial String");
        assert_eq!(dram.location.to_string(), "Debug DRAM Location String");
        assert_eq!(
            dram.flags,
            ControllerFlags::LOCAL
                .union(ControllerFlags::MANUALLY_CONFIGURABLE_NAME)
                .union(ControllerFlags::MANUALLY_CONFIGURABLE_DEVICE_SPECIFIC)
        );

        assert_eq!(dram.active_mode, 0);
        let expected_value = match protocol {
            ProtocolVersion::V5 => Some(0),
            ProtocolVersion::V6 => None,
        };
        assert_eq!(
            dram.modes,
            [ModeDescription {
                name: "Direct".into(),
                value: expected_value,
                flags: ModeFlags::HAS_PER_LED_COLOR,
                speed_min: 0,
                speed_max: 0,
                brightness_min: 0,
                brightness_max: 0,
                colors_min: 0,
                colors_max: 0,
                speed: 0,
                brightness: 0,
                direction: Direction::LEFT,
                color_mode: ColorMode::PerLed,
                colors: vec![],
            }]
        );

        let zones: Vec<(String, ZoneType, u32)> = dram
            .zones
            .iter()
            .map(|z| (z.name.to_string(), z.zone_type, z.leds_count))
            .collect();
        assert_eq!(
            zones,
            [
                ("Single Zone".to_owned(), ZoneType::Single, 1),
                ("Linear Zone".to_owned(), ZoneType::Linear, 10)
            ]
        );
        assert_eq!(
            dram.zones[0].flags,
            ZoneFlags::MANUALLY_CONFIGURABLE_DEVICE_SPECIFIC
        );
        assert!(
            dram.zones
                .iter()
                .all(|z| z.matrix.is_none() && z.segments.is_empty())
        );

        let mut leds = vec!["Single LED".to_owned()];
        leds.extend((0..10).map(|i| format!("Linear LED {i}")));
        assert_eq!(names(dram.leds.iter().map(|l| l.name.clone())), leds);
        assert!(
            dram.leds
                .iter()
                .all(|l| l.value == expected_value.map(|v| v as u32))
        );
        assert_eq!(dram.colors, vec![BASE01; 11], "the owner's nordic base01");
        assert_eq!(dram.led_display_names, vec![WireString::default(); 11]);

        match protocol {
            ProtocolVersion::V5 => {
                assert!(dram.v6.is_none());
                assert!(dram.zones.iter().all(|z| z.v6.is_none()));
            }
            ProtocolVersion::V6 => {
                let v6 = dram.v6.as_ref().unwrap();
                assert_eq!(v6.display_name, WireString::default());
                let configuration: serde_json::Value =
                    serde_json::from_slice(v6.configuration.as_bytes()).unwrap();
                assert_eq!(configuration["configuration"]["test_int"], 12345);
                assert_eq!(
                    configuration["zones"][0]["configuration"]["color_order"],
                    "RGB"
                );
                for zone in &dram.zones {
                    let zone = zone.v6.as_ref().unwrap();
                    assert_eq!(zone.active_mode, -1, "no per-zone mode");
                    assert!(zone.modes.is_empty());
                    assert_eq!(zone.display_name, WireString::default());
                }
            }
        }
    }
}

#[test]
fn the_ddp_device_decodes_exactly() {
    for protocol in PROTOCOLS {
        let ddp = load(protocol).one("ENE DRAM Wire");
        assert_eq!(ddp.device_type.name(), Some("ledstrip"));
        assert_eq!(
            ddp.description.to_string(),
            "Distributed Display Protocol Device"
        );
        assert_eq!(ddp.location.to_string(), "DDP: 127.0.0.1:4048");
        assert_eq!(ddp.vendor, WireString::default());
        assert_eq!(ddp.flags, ControllerFlags::LOCAL);
        assert_eq!(ddp.modes.len(), 1);
        let direct = &ddp.modes[0];
        assert_eq!(direct.name.to_string(), "Direct");
        assert_eq!(
            direct.flags,
            ModeFlags::HAS_BRIGHTNESS.union(ModeFlags::HAS_PER_LED_COLOR)
        );
        assert_eq!(
            (
                direct.brightness_min,
                direct.brightness_max,
                direct.brightness
            ),
            (0, 100, 100)
        );
        assert_eq!(direct.color_mode, ColorMode::PerLed);
        assert_eq!(ddp.zones.len(), 1);
        let zone = &ddp.zones[0];
        assert_eq!(zone.name.to_string(), "ENE DRAM Wire");
        assert_eq!(zone.zone_type, ZoneType::Linear);
        assert_eq!((zone.leds_min, zone.leds_max, zone.leds_count), (8, 8, 8));
        assert_eq!(
            zone.flags,
            ZoneFlags::MANUALLY_CONFIGURABLE_NAME
                .union(ZoneFlags::MANUALLY_CONFIGURABLE_TYPE)
                .union(ZoneFlags::MANUALLY_CONFIGURABLE_MATRIX_MAP)
                .union(ZoneFlags::MANUALLY_CONFIGURABLE_SEGMENTS)
        );
        assert_eq!(
            names(ddp.leds.iter().map(|l| l.name.clone())),
            (1..=8)
                .map(|i| format!("ENE DRAM Wire LED {i}"))
                .collect::<Vec<_>>()
        );
        assert_eq!(ddp.colors, vec![BASE01; 8]);
        assert!(ddp.led_display_names.is_empty());
    }
}

#[test]
fn the_govee_device_decodes_exactly() {
    for protocol in PROTOCOLS {
        let govee = load(protocol).one("Govee ");
        assert_eq!(govee.device_type.name(), Some("light"));
        assert_eq!(govee.name.to_string(), "Govee ", "no SKU answered the scan");
        assert_eq!(govee.vendor.to_string(), "Govee");
        assert_eq!(govee.location.to_string(), "IP: 127.0.0.1");
        assert_eq!(
            govee.version.to_string(),
            "BLE Hardware Version: \r\nBLE Software Version: \r\n\
             WiFi Hardware Version: \r\nWiFI Software Version: \r\n"
        );
        assert_eq!(govee.flags, ControllerFlags::LOCAL);
        assert_eq!(govee.active_mode, 0);
        let value = |v: i32| match protocol {
            ProtocolVersion::V5 => Some(v),
            ProtocolVersion::V6 => None,
        };
        assert_eq!(
            govee.modes,
            [
                ModeDescription {
                    name: "Static".into(),
                    value: value(0),
                    flags: ModeFlags::HAS_BRIGHTNESS.union(ModeFlags::HAS_MODE_SPECIFIC_COLOR),
                    speed_min: 0,
                    speed_max: 0,
                    brightness_min: 0,
                    brightness_max: 100,
                    colors_min: 1,
                    colors_max: 1,
                    speed: 0,
                    brightness: 100,
                    direction: Direction::LEFT,
                    color_mode: ColorMode::ModeSpecific,
                    colors: vec![BASE0D],
                },
                ModeDescription {
                    name: "Direct".into(),
                    value: value(1),
                    flags: ModeFlags::HAS_BRIGHTNESS.union(ModeFlags::HAS_PER_LED_COLOR),
                    speed_min: 0,
                    speed_max: 0,
                    brightness_min: 0,
                    brightness_max: 100,
                    colors_min: 0,
                    colors_max: 0,
                    speed: 0,
                    brightness: 100,
                    direction: Direction::LEFT,
                    color_mode: ColorMode::PerLed,
                    colors: vec![],
                },
            ]
        );
        assert_eq!(govee.zones.len(), 1);
        assert_eq!(govee.zones[0].name.to_string(), "Govee Strip");
        assert_eq!(govee.zones[0].leds_count, 1);
        assert_eq!(
            names(govee.leds.iter().map(|l| l.name.clone())),
            ["Govee LED 0"]
        );
        assert_eq!(
            govee.colors,
            [BLACK],
            "Static carries its colour in the mode"
        );
    }
}

#[test]
fn the_debug_keyboard_matrices_decode_exactly() {
    for protocol in PROTOCOLS {
        let keyboard = load(protocol).one("Debug Keyboard");
        assert_eq!(keyboard.device_type.name(), Some("keyboard"));
        let zones: Vec<(String, ZoneType, u32)> = keyboard
            .zones
            .iter()
            .map(|z| (z.name.to_string(), z.zone_type, z.leds_count))
            .collect();
        assert_eq!(
            zones,
            [
                ("Single Zone".to_owned(), ZoneType::Single, 1),
                ("Linear Zone".to_owned(), ZoneType::Linear, 10),
                ("Keyboard Zone".to_owned(), ZoneType::Matrix, 104),
                ("Underglow Zone".to_owned(), ZoneType::Matrix, 30),
            ]
        );
        assert_eq!(
            keyboard.zones[2].flags,
            ZoneFlags::MANUALLY_CONFIGURABLE_DEVICE_SPECIFIC.union(ZoneFlags::GEOMETRY_MAY_CHANGE)
        );

        // The key matrix places every key exactly once in a 6x21 grid; the
        // other cells hold no LED.
        let keys = keyboard.zones[2].matrix.as_ref().unwrap();
        assert_eq!((keys.height, keys.width), (6, 21));
        assert_eq!(keys.map.len(), 126);
        let mut placed: Vec<u32> = keys.map.iter().copied().filter(|&c| c != NO_LED).collect();
        placed.sort_unstable();
        assert_eq!(placed, (0..104).collect::<Vec<_>>());
        assert_eq!(keys.map.iter().filter(|&&c| c == NO_LED).count(), 22);

        let underglow = keyboard.zones[3].matrix.as_ref().unwrap();
        assert_eq!((underglow.height, underglow.width), (3, 10));
        assert_eq!(underglow.map, (0..30).collect::<Vec<_>>());

        let led_names = names(keyboard.leds.iter().map(|l| l.name.clone()));
        assert_eq!(led_names.len(), 145);
        assert_eq!(led_names[11], "Key: Escape");
        assert_eq!(led_names[114], "Key: Number Pad .");
        assert_eq!(led_names[115], "Underglow LED 0");
        assert_eq!(led_names[144], "Underglow LED 29");
        assert_eq!(
            keyboard.colors,
            vec![BLACK; 145],
            "no device spec selects it"
        );
        assert_eq!(keyboard.led_display_names.len(), 145);
    }
}

#[test]
fn plans_for_the_captured_controllers_match_what_the_server_holds() {
    for protocol in PROTOCOLS {
        let capture = load(protocol);
        let dram = capture.one("ENE DRAM Wire");
        let stick = &capture.named("ENE DRAM")[0];
        let govee = capture.one("Govee ");
        let keyboard = capture.one("Debug Keyboard");

        // The owner's selectors, on the real display names.
        let ene = Selector::new("ENE DRAM", "Direct").unwrap();
        assert!(ene.matches(&dram) && ene.matches(stick));
        assert!(!ene.matches(&govee) && !ene.matches(&keyboard));
        assert!(Selector::new("govee", "static").unwrap().matches(&govee));

        // Direct is active and per-LED: a reconcile writes only the LEDs, one
        // colour per LED; the captured state is what that plan expects.
        let plan = select::plan(stick, "Direct", BASE01, ApplyKind::Reconcile).unwrap();
        assert_eq!(plan.path(), ColourPath::PerLed);
        assert_eq!(plan.mode_write(), None);
        assert_eq!(plan.led_write(), Some(vec![BASE01; 11].as_slice()));
        select::confirm(&plan.expect, stick).unwrap();
        select::confirm(
            &select::plan(&dram, "direct", BASE01, ApplyKind::Reconcile)
                .unwrap()
                .expect,
            &dram,
        )
        .unwrap();

        // A forced apply echoes the server's own Direct block, which its
        // acceptance rule takes, and encodes to the captured bytes.
        let forced = select::plan(stick, "Direct", BASE01, ApplyKind::Forced).unwrap();
        let echo = forced.mode_write().unwrap();
        select::check_acceptance(&stick.modes[0], echo).unwrap();
        let raw = capture
            .payloads()
            .find(|(name, _)| *name == "ENE DRAM")
            .unwrap()
            .1;
        let mut w = Writer::new();
        codec::encode_mode(&mut w, echo, protocol).unwrap();
        assert_eq!(w.into_inner(), mode_blocks(raw, protocol)[0].1);

        // Static is mode-specific with exactly one colour: UPDATEMODE alone,
        // brightness echoed at 100.
        let plan = select::plan(&govee, "Static", BASE0D, ApplyKind::Reconcile).unwrap();
        assert_eq!(plan.path(), ColourPath::ModeSpecific);
        let Writes::ModeSpecific { mode } = &plan.writes else {
            panic!("{:?}", plan.writes);
        };
        assert_eq!(mode.colors, [BASE0D]);
        assert_eq!(mode.brightness, 100);
        assert_eq!(
            *mode, govee.modes[0],
            "the server holds exactly the planned block"
        );
        select::confirm(&plan.expect, &govee).unwrap();

        // A mode the controller lacks is named with the modes it has.
        assert_eq!(
            select::plan(&keyboard, "Static", BASE01, ApplyKind::Reconcile),
            Err(PlanError::UnknownMode {
                requested: "Static".into(),
                available: vec!["Direct".into()],
            })
        );
    }
}

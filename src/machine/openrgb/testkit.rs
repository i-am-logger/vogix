//! Test support for the OpenRGB client: the server-side block layout (so tests
//! can build description payloads), builders for the frames a server sends,
//! controller fixtures, a seeded generator of arbitrary controllers, an
//! allocation probe, and a loopback port that refuses connections.
//!
//! The frame builders script byte streams for the state-machine tests in
//! `session`; they are not a server and prove nothing about protocol
//! compatibility, which the real-server VM checks establish.

use super::codec::encode_mode;
use super::model::{
    AckStatus, ColorMode, ControllerDescription, ControllerFlags, ControllerV6, DeviceType,
    Direction, LedDescription, MatrixMap, ModeDescription, ModeFlags, ProtocolVersion,
    SegmentDescription, SegmentFlags, SegmentV6, UpdateReason, ZoneDescription, ZoneFlags,
    ZoneType, ZoneV6,
};
use super::wire::{Frame, PacketId, RgbColor, WireString, Writer};
use std::io;
use std::net::Ipv4Addr;
use std::os::fd::{AsFd, AsRawFd, BorrowedFd, FromRawFd, OwnedFd};

/// Largest-allocation probe: a global allocator that, while armed on the
/// current thread, records the largest single allocation or reallocation size
/// requested.
pub mod alloc_probe {
    use std::alloc::{GlobalAlloc, Layout, System};
    use std::cell::Cell;

    struct Probe;

    thread_local! {
        static ARMED: Cell<bool> = const { Cell::new(false) };
        static PEAK: Cell<usize> = const { Cell::new(0) };
    }

    fn record(size: usize) {
        let armed = ARMED.try_with(Cell::get).unwrap_or(false);
        if armed {
            let _ = PEAK.try_with(|peak| peak.set(peak.get().max(size)));
        }
    }

    // SAFETY: every call forwards to `System` with the caller's arguments; the
    // probe only reads and writes const-initialised thread-locals, which never
    // allocate.
    unsafe impl GlobalAlloc for Probe {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            record(layout.size());
            // SAFETY: forwarded unchanged.
            unsafe { System.alloc(layout) }
        }

        unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
            record(layout.size());
            // SAFETY: forwarded unchanged.
            unsafe { System.alloc_zeroed(layout) }
        }

        unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
            // SAFETY: forwarded unchanged.
            unsafe { System.dealloc(ptr, layout) }
        }

        unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
            record(new_size);
            // SAFETY: forwarded unchanged.
            unsafe { System.realloc(ptr, layout, new_size) }
        }
    }

    #[global_allocator]
    static PROBE: Probe = Probe;

    /// Run `f` and return its result with the largest single allocation it
    /// requested on this thread.
    pub fn peak_allocation<R>(f: impl FnOnce() -> R) -> (R, usize) {
        PEAK.with(|peak| peak.set(0));
        ARMED.with(|armed| armed.set(true));
        let result = f();
        ARMED.with(|armed| armed.set(false));
        (result, PEAK.with(Cell::get))
    }
}

fn encode_matrix(w: &mut Writer, matrix: Option<&MatrixMap>) {
    match matrix {
        None => w.u16(0),
        Some(matrix) => {
            // The server computes this length as a u16 that wraps.
            let len = 8 + 4 * u64::from(matrix.height) * u64::from(matrix.width);
            w.u16((len & 0xffff) as u16);
            w.u32(matrix.height);
            w.u32(matrix.width);
            for cell in &matrix.map {
                w.u32(*cell);
            }
        }
    }
}

fn encode_segment(w: &mut Writer, segment: &SegmentDescription, version: ProtocolVersion) {
    w.string_u16(&segment.name, "segment_name").unwrap();
    w.u32(segment.segment_type.to_wire());
    w.u32(segment.start_idx);
    w.u32(segment.leds_count);
    if version == ProtocolVersion::V6 {
        let v6 = segment.v6.as_ref().expect("protocol 6 segment fields");
        encode_matrix(w, v6.matrix.as_ref());
        w.u32(v6.flags.bits());
    }
}

fn encode_zone(w: &mut Writer, zone: &ZoneDescription, version: ProtocolVersion) {
    w.string_u16(&zone.name, "zone_name").unwrap();
    w.u32(zone.zone_type.to_wire());
    w.u32(zone.leds_min);
    w.u32(zone.leds_max);
    w.u32(zone.leds_count);
    encode_matrix(w, zone.matrix.as_ref());
    w.count_u16(zone.segments.len(), "segments").unwrap();
    for segment in &zone.segments {
        encode_segment(w, segment, version);
    }
    w.u32(zone.flags.bits());
    if version == ProtocolVersion::V6 {
        let v6 = zone.v6.as_ref().expect("protocol 6 zone fields");
        w.i32(v6.active_mode);
        w.count_u16(v6.modes.len(), "zone modes").unwrap();
        for mode in &v6.modes {
            encode_mode(w, mode, version).unwrap();
        }
        w.string_u16(&v6.display_name, "zone_display_name").unwrap();
    }
}

/// One Device Data block laid out as the server writes it.
pub fn encode_device(
    w: &mut Writer,
    description: &ControllerDescription,
    version: ProtocolVersion,
) {
    w.i32(description.device_type.0);
    for text in [
        &description.name,
        &description.vendor,
        &description.description,
        &description.version,
        &description.serial,
        &description.location,
    ] {
        w.string_u16(text, "string").unwrap();
    }
    w.count_u16(description.modes.len(), "modes").unwrap();
    w.i32(description.active_mode);
    for mode in &description.modes {
        encode_mode(w, mode, version).unwrap();
    }
    w.count_u16(description.zones.len(), "zones").unwrap();
    for zone in &description.zones {
        encode_zone(w, zone, version);
    }
    w.count_u16(description.leds.len(), "leds").unwrap();
    for led in &description.leds {
        w.string_u16(&led.name, "led_name").unwrap();
        if version == ProtocolVersion::V5 {
            w.u32(led.value.expect("protocol 5 LED value"));
        }
    }
    w.count_u16(description.colors.len(), "colors").unwrap();
    for colour in &description.colors {
        w.u32(colour.to_wire());
    }
    w.count_u16(description.led_display_names.len(), "led display names")
        .unwrap();
    for name in &description.led_display_names {
        w.string_u16(name, "led_display_name").unwrap();
    }
    w.u32(description.flags.bits());
    if version == ProtocolVersion::V6 {
        let v6 = description
            .v6
            .as_ref()
            .expect("protocol 6 controller fields");
        w.string_u16(&v6.display_name, "display_name").unwrap();
        w.string_u32(&v6.configuration, "configuration").unwrap();
    }
}

fn with_data_size(body: Vec<u8>) -> Vec<u8> {
    let size = (body.len() + 4) as u32;
    let mut payload = size.to_le_bytes().to_vec();
    payload.extend(body);
    payload
}

/// A `REQUEST_CONTROLLER_DATA` reply payload.
pub fn controller_data_payload(
    description: &ControllerDescription,
    version: ProtocolVersion,
) -> Vec<u8> {
    let mut w = Writer::new();
    encode_device(&mut w, description, version);
    with_data_size(w.into_inner())
}

/// A `SIGNALUPDATE` payload with the `UPDATELEDS` reason.
pub fn signal_update_leds_payload(colours: &[RgbColor]) -> Vec<u8> {
    let mut w = Writer::new();
    w.u32(UpdateReason::UpdateLeds.to_wire());
    w.count_u16(colours.len(), "colors").unwrap();
    for colour in colours {
        w.u32(colour.to_wire());
    }
    with_data_size(w.into_inner())
}

/// A `SIGNALUPDATE` payload carrying the whole description.
pub fn signal_update_device_payload(
    reason: UpdateReason,
    description: &ControllerDescription,
    version: ProtocolVersion,
) -> Vec<u8> {
    let mut w = Writer::new();
    w.u32(reason.to_wire());
    encode_device(&mut w, description, version);
    with_data_size(w.into_inner())
}

/// Scripted server frames.
pub mod server {
    use super::*;

    pub fn frame(dev_id: u32, pkt_id: PacketId, payload: Vec<u8>) -> Vec<u8> {
        Frame {
            dev_id,
            pkt_id,
            payload,
        }
        .encode()
        .unwrap()
    }

    pub fn version_reply(max: u32) -> Vec<u8> {
        frame(
            0,
            PacketId::RequestProtocolVersion,
            max.to_le_bytes().to_vec(),
        )
    }

    pub fn server_name(name: &str) -> Vec<u8> {
        let mut payload = name.as_bytes().to_vec();
        payload.push(0);
        frame(0, PacketId::SetServerName, payload)
    }

    pub fn server_flags(bits: u32) -> Vec<u8> {
        frame(0, PacketId::SetServerFlags, bits.to_le_bytes().to_vec())
    }

    pub fn ack(dev_id: u32, acked: PacketId, status: AckStatus) -> Vec<u8> {
        let mut payload = acked.to_wire().to_le_bytes().to_vec();
        payload.extend(status.to_wire().to_le_bytes());
        frame(dev_id, PacketId::Ack, payload)
    }

    pub fn ok(dev_id: u32, acked: PacketId) -> Vec<u8> {
        ack(dev_id, acked, AckStatus::Ok)
    }

    pub fn count_v5(count: u32) -> Vec<u8> {
        frame(
            0,
            PacketId::RequestControllerCount,
            count.to_le_bytes().to_vec(),
        )
    }

    pub fn count_v6(ids: &[u32]) -> Vec<u8> {
        let mut payload = (ids.len() as u32).to_le_bytes().to_vec();
        for id in ids {
            payload.extend(id.to_le_bytes());
        }
        frame(0, PacketId::RequestControllerCount, payload)
    }

    pub fn controller_data(
        dev_id: u32,
        description: &ControllerDescription,
        version: ProtocolVersion,
    ) -> Vec<u8> {
        frame(
            dev_id,
            PacketId::RequestControllerData,
            controller_data_payload(description, version),
        )
    }

    pub fn device_list_updated() -> Vec<u8> {
        frame(0, PacketId::DeviceListUpdated, Vec::new())
    }

    pub fn detection_started() -> Vec<u8> {
        frame(0, PacketId::DetectionStarted, Vec::new())
    }

    pub fn detection_complete() -> Vec<u8> {
        frame(0, PacketId::DetectionComplete, Vec::new())
    }

    pub fn detection_progress(percent: u32, text: &str) -> Vec<u8> {
        let mut w = Writer::new();
        w.u32(percent);
        w.string_u16(&WireString::from(text), "detection_string")
            .unwrap();
        frame(
            0,
            PacketId::DetectionProgressChanged,
            with_data_size(w.into_inner()),
        )
    }

    pub fn signal_update_leds(dev_id: u32, colours: &[RgbColor]) -> Vec<u8> {
        frame(
            dev_id,
            PacketId::SignalUpdate,
            signal_update_leds_payload(colours),
        )
    }

    pub fn signal_update_device(
        dev_id: u32,
        reason: UpdateReason,
        description: &ControllerDescription,
        version: ProtocolVersion,
    ) -> Vec<u8> {
        frame(
            dev_id,
            PacketId::SignalUpdate,
            signal_update_device_payload(reason, description, version),
        )
    }
}

fn mode(
    version: ProtocolVersion,
    name: &str,
    value: i32,
    flags: ModeFlags,
    color_mode: ColorMode,
) -> ModeDescription {
    ModeDescription {
        name: name.into(),
        value: (version == ProtocolVersion::V5).then_some(value),
        flags,
        speed_min: 0,
        speed_max: 0,
        brightness_min: 0,
        brightness_max: 0,
        colors_min: 0,
        colors_max: 0,
        speed: 0,
        brightness: 0,
        direction: Direction::LEFT,
        color_mode,
        colors: Vec::new(),
    }
}

/// A plain "Static" per-LED mode with every numeric field zero.
pub fn static_per_led_mode(version: ProtocolVersion) -> ModeDescription {
    mode(
        version,
        "Static",
        0,
        ModeFlags::HAS_PER_LED_COLOR,
        ColorMode::PerLed,
    )
}

fn linear_zone(version: ProtocolVersion, name: &str, leds: u32) -> ZoneDescription {
    ZoneDescription {
        name: name.into(),
        zone_type: ZoneType::Linear,
        leds_min: leds,
        leds_max: leds,
        leds_count: leds,
        matrix: None,
        segments: Vec::new(),
        flags: ZoneFlags::default(),
        v6: (version == ProtocolVersion::V6).then(|| ZoneV6 {
            active_mode: -1,
            modes: Vec::new(),
            display_name: WireString::default(),
        }),
    }
}

fn controller(
    version: ProtocolVersion,
    name: &str,
    device_type: i32,
    modes: Vec<ModeDescription>,
    active_mode: i32,
    zones: Vec<ZoneDescription>,
    colour: RgbColor,
) -> ControllerDescription {
    let led_count: u32 = zones.iter().map(|zone| zone.leds_count).sum();
    ControllerDescription {
        device_type: DeviceType(device_type),
        name: name.into(),
        vendor: "Vendor".into(),
        description: "Test controller".into(),
        version: "1.0".into(),
        serial: WireString::default(),
        location: "I2C: /dev/i2c-1, address 0x70".into(),
        active_mode,
        modes,
        zones,
        leds: (0..led_count)
            .map(|index| LedDescription {
                name: WireString::from(format!("LED {index}").as_str()),
                value: (version == ProtocolVersion::V5).then_some(index),
            })
            .collect(),
        colors: vec![colour; led_count as usize],
        led_display_names: Vec::new(),
        flags: ControllerFlags::LOCAL,
        v6: (version == ProtocolVersion::V6).then(|| ControllerV6 {
            display_name: WireString::default(),
            configuration: WireString::default(),
        }),
    }
}

/// ENE DRAM as `RGBController_ENESMBus` lays it out: Direct, Off, Static and
/// Breathing (whose speed range runs from 4, slowest, down to 0, fastest), one
/// linear zone of 8 LEDs, all LEDs black.
pub fn ene_dram(version: ProtocolVersion, active_mode: i32) -> ControllerDescription {
    let mut breathing = mode(
        version,
        "Breathing",
        2,
        ModeFlags::HAS_RANDOM_COLOR
            .union(ModeFlags::HAS_PER_LED_COLOR)
            .union(ModeFlags::HAS_SPEED),
        ColorMode::PerLed,
    );
    breathing.speed_min = 4;
    breathing.speed_max = 0;
    breathing.speed = 2;
    controller(
        version,
        "ENE DRAM",
        1,
        vec![
            mode(
                version,
                "Direct",
                0xff,
                ModeFlags::HAS_PER_LED_COLOR,
                ColorMode::PerLed,
            ),
            mode(version, "Off", 0, ModeFlags::default(), ColorMode::None),
            static_per_led_mode(version),
            breathing,
        ],
        active_mode,
        vec![linear_zone(version, "DRAM", 8)],
        RgbColor::from_rgb(0, 0, 0),
    )
}

/// A Govee light as `RGBController_Govee` lays it out: Direct (per-LED) and
/// Static (one mode-specific colour), both with brightness 0..=100 at 100.
pub fn govee(version: ProtocolVersion) -> ControllerDescription {
    let mut direct = mode(
        version,
        "Direct",
        0,
        ModeFlags::HAS_PER_LED_COLOR.union(ModeFlags::HAS_BRIGHTNESS),
        ColorMode::PerLed,
    );
    direct.brightness_max = 100;
    direct.brightness = 100;
    let mut fixed = mode(
        version,
        "Static",
        1,
        ModeFlags::HAS_MODE_SPECIFIC_COLOR.union(ModeFlags::HAS_BRIGHTNESS),
        ColorMode::ModeSpecific,
    );
    fixed.brightness_max = 100;
    fixed.brightness = 100;
    fixed.colors_min = 1;
    fixed.colors_max = 1;
    fixed.colors = vec![RgbColor::from_rgb(0, 0, 0)];
    controller(
        version,
        "Govee H6199",
        11,
        vec![direct, fixed],
        0,
        vec![linear_zone(version, "Govee", 1)],
        RgbColor::from_rgb(0, 0, 0),
    )
}

/// A keyboard with a matrix zone, segments, LED display names and, at protocol
/// 6, a configured display name, per-zone modes and segment matrices.
pub fn keyboard_with_matrix_and_segments(version: ProtocolVersion) -> ControllerDescription {
    let mut keys = linear_zone(version, "Keys", 6);
    keys.zone_type = ZoneType::Matrix;
    keys.matrix = Some(MatrixMap {
        height: 2,
        width: 3,
        map: vec![0, 1, 2, 3, 4, 5],
    });
    keys.flags = ZoneFlags::MANUALLY_CONFIGURABLE_NAME;
    keys.segments = vec![SegmentDescription {
        name: "Top row".into(),
        segment_type: ZoneType::Linear,
        start_idx: 0,
        leds_count: 3,
        v6: (version == ProtocolVersion::V6).then(|| SegmentV6 {
            matrix: Some(MatrixMap {
                height: 1,
                width: 3,
                map: vec![0, 1, 2],
            }),
            flags: SegmentFlags::GROUP_START,
        }),
    }];
    if let Some(v6) = keys.v6.as_mut() {
        v6.active_mode = 0;
        v6.modes = vec![mode(
            version,
            "Wave",
            3,
            ModeFlags::HAS_DIRECTION_LR,
            ColorMode::None,
        )];
        v6.display_name = "Main keys".into();
    }
    let mut keyboard = controller(
        version,
        "Debug Keyboard",
        5,
        vec![
            mode(
                version,
                "Direct",
                0,
                ModeFlags::HAS_PER_LED_COLOR,
                ColorMode::PerLed,
            ),
            mode(
                version,
                "Wave",
                3,
                ModeFlags::HAS_DIRECTION_LR,
                ColorMode::None,
            ),
        ],
        0,
        vec![keys],
        RgbColor::from_rgb(0x10, 0x20, 0x30),
    );
    keyboard.led_display_names = (0..6)
        .map(|index| WireString::from(format!("Key {index}").as_str()))
        .collect();
    if let Some(v6) = keyboard.v6.as_mut() {
        v6.display_name = "My keyboard".into();
        v6.configuration = "{\"layout\":\"ansi\"}".into();
        keyboard.flags = keyboard
            .flags
            .union(ControllerFlags::MANUALLY_CONFIGURED_NAME);
    }
    keyboard
}

/// splitmix64: a small deterministic generator for seeded structures.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(seed)
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    pub fn u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }

    pub fn below(&mut self, bound: u32) -> u32 {
        (self.next_u64() % u64::from(bound.max(1))) as u32
    }

    pub fn string(&mut self, max_len: u32) -> WireString {
        let len = self.below(max_len + 1);
        WireString::from_bytes((0..len).map(|_| 1 + self.below(255) as u8).collect())
    }

    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
}

/// A mode with every field drawn from `seed`.
pub fn arbitrary_mode(version: ProtocolVersion, seed: u64) -> ModeDescription {
    let mut rng = Rng::new(seed);
    random_mode(&mut rng, version)
}

fn random_mode(rng: &mut Rng, version: ProtocolVersion) -> ModeDescription {
    let colours = rng.below(6);
    ModeDescription {
        name: rng.string(16),
        value: (version == ProtocolVersion::V5).then(|| rng.u32() as i32),
        flags: ModeFlags::from_bits(rng.u32()),
        speed_min: rng.u32(),
        speed_max: rng.u32(),
        brightness_min: rng.u32(),
        brightness_max: rng.u32(),
        colors_min: rng.u32(),
        colors_max: rng.u32(),
        speed: rng.u32(),
        brightness: rng.u32(),
        direction: Direction(rng.u32()),
        color_mode: ColorMode::from_wire(rng.below(6)),
        colors: (0..colours)
            .map(|_| RgbColor::from_wire(rng.u32()))
            .collect(),
    }
}

fn random_matrix(rng: &mut Rng) -> Option<MatrixMap> {
    rng.bool().then(|| {
        let height = rng.below(4);
        let width = rng.below(4);
        MatrixMap {
            height,
            width,
            map: (0..height * width).map(|_| rng.u32()).collect(),
        }
    })
}

/// A controller with every field drawn from `seed`.
pub fn arbitrary_controller(version: ProtocolVersion, seed: u64) -> ControllerDescription {
    let mut rng = Rng::new(seed);
    let v6 = version == ProtocolVersion::V6;
    let modes = (0..rng.below(4))
        .map(|_| random_mode(&mut rng, version))
        .collect();
    let zones = (0..rng.below(3))
        .map(|_| ZoneDescription {
            name: rng.string(12),
            zone_type: ZoneType::from_wire(rng.below(9)),
            leds_min: rng.u32(),
            leds_max: rng.u32(),
            leds_count: rng.u32(),
            matrix: random_matrix(&mut rng),
            segments: (0..rng.below(3))
                .map(|_| SegmentDescription {
                    name: rng.string(8),
                    segment_type: ZoneType::from_wire(rng.below(9)),
                    start_idx: rng.u32(),
                    leds_count: rng.u32(),
                    v6: v6.then(|| SegmentV6 {
                        matrix: random_matrix(&mut rng),
                        flags: SegmentFlags::from_bits(rng.u32()),
                    }),
                })
                .collect(),
            flags: ZoneFlags::from_bits(rng.u32()),
            v6: v6.then(|| ZoneV6 {
                active_mode: rng.u32() as i32,
                modes: (0..rng.below(3))
                    .map(|_| random_mode(&mut rng, version))
                    .collect(),
                display_name: rng.string(8),
            }),
        })
        .collect();
    let leds = rng.below(6);
    ControllerDescription {
        device_type: DeviceType(rng.u32() as i32),
        name: rng.string(16),
        vendor: rng.string(8),
        description: rng.string(8),
        version: rng.string(8),
        serial: rng.string(8),
        location: rng.string(8),
        active_mode: rng.u32() as i32,
        modes,
        zones,
        leds: (0..leds)
            .map(|_| LedDescription {
                name: rng.string(6),
                value: (!v6).then(|| rng.u32()),
            })
            .collect(),
        colors: (0..rng.below(8))
            .map(|_| RgbColor::from_wire(rng.u32()))
            .collect(),
        led_display_names: (0..rng.below(4)).map(|_| rng.string(6)).collect(),
        flags: ControllerFlags::from_bits(rng.u32()),
        v6: v6.then(|| ControllerV6 {
            display_name: rng.string(12),
            configuration: rng.string(24),
        }),
    }
}

/// A loopback TCP port that refuses every connection while this value lives.
///
/// The port is held by a socket that is bound and never listens, without
/// `SO_REUSEADDR`: a connect to it is refused, and nothing else can bind the
/// port meanwhile. A listener that is bound and then dropped gives neither: a
/// child another test forks holds a copy of the listening socket until its
/// exec, which keeps the port accepting, and once the socket is closed any
/// other bind may take the port.
pub struct RefusingPort {
    socket: OwnedFd,
    port: u16,
}

impl RefusingPort {
    pub fn new() -> Self {
        // SAFETY: plain socket(2) call.
        let fd = unsafe { libc::socket(libc::AF_INET, libc::SOCK_STREAM | libc::SOCK_CLOEXEC, 0) };
        assert!(fd >= 0, "socket: {}", io::Error::last_os_error());
        // SAFETY: socket returned a fresh fd that nothing else owns.
        let socket = unsafe { OwnedFd::from_raw_fd(fd) };
        // SAFETY: an all-zero sockaddr_in is valid; the fields are then set.
        let mut addr: libc::sockaddr_in = unsafe { std::mem::zeroed() };
        addr.sin_family = libc::AF_INET as libc::sa_family_t;
        addr.sin_addr.s_addr = u32::from(Ipv4Addr::LOCALHOST).to_be();
        let mut len = std::mem::size_of::<libc::sockaddr_in>() as libc::socklen_t;
        // SAFETY: `addr` is a valid sockaddr_in of the length given.
        let rc = unsafe { libc::bind(socket.as_raw_fd(), (&raw const addr).cast(), len) };
        assert_eq!(rc, 0, "bind: {}", io::Error::last_os_error());
        // SAFETY: `addr` and `len` are valid for writes of a sockaddr_in.
        let rc =
            unsafe { libc::getsockname(socket.as_raw_fd(), (&raw mut addr).cast(), &raw mut len) };
        assert_eq!(rc, 0, "getsockname: {}", io::Error::last_os_error());
        Self {
            socket,
            port: u16::from_be(addr.sin_port),
        }
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    /// The socket holding the port.
    pub fn socket(&self) -> BorrowedFd<'_> {
        self.socket.as_fd()
    }
}

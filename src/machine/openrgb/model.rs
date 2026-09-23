//! Typed model of what the OpenRGB SDK carries: the negotiated protocol
//! version, the flag and enum fields, and the controller description blocks.
//!
//! Numeric values follow `NetworkProtocol.h` and
//! `RGBController/RGBControllerInterface.h` of the OpenRGB tree vogix's server
//! package is built from. Fields present only at some protocol versions are
//! `Option`s (or `v6` groups) that are `Some` exactly when the block was decoded
//! at a version that carries them, which is what lets a block re-encode byte for
//! byte at the version it was read at.

use super::wire::{RgbColor, WireString};
use serde::{Deserialize, Serialize, Serializer};
use std::fmt;

/// SDK protocol versions vogix speaks.
///
/// Protocol 5 addresses controllers by list index and has no acknowledgements;
/// protocol 6 adds stable controller ids, ACKs, detection notifications and
/// `SIGNALUPDATE` (`Documentation/OpenRGBSDK.md`, "Protocol Versions").
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "u32", into = "u32")]
pub enum ProtocolVersion {
    V5,
    V6,
}

impl ProtocolVersion {
    /// The protocol number on the wire.
    pub const fn number(self) -> u32 {
        match self {
            Self::V5 => 5,
            Self::V6 => 6,
        }
    }

    /// The version both sides speak: the lower of the server's highest version
    /// and `client_max`. The server stores the same minimum for this client
    /// (`NetworkServer.cpp`, `ProcessRequest_ClientProtocolVersion` clamps the
    /// client's value to its own), so both ends agree without a second exchange.
    pub fn negotiate(server_max: u32, client_max: Self) -> Result<Self, UnsupportedProtocol> {
        Self::try_from(server_max.min(client_max.number()))
            .map_err(|_| UnsupportedProtocol { server_max })
    }
}

impl TryFrom<u32> for ProtocolVersion {
    type Error = UnsupportedProtocol;

    fn try_from(number: u32) -> Result<Self, Self::Error> {
        match number {
            5 => Ok(Self::V5),
            6 => Ok(Self::V6),
            other => Err(UnsupportedProtocol { server_max: other }),
        }
    }
}

impl From<ProtocolVersion> for u32 {
    fn from(version: ProtocolVersion) -> Self {
        version.number()
    }
}

impl fmt::Display for ProtocolVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.number())
    }
}

/// The server's highest protocol version is below 5.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("OpenRGB speaks SDK protocol {server_max}; vogix speaks protocols 5 and 6")]
pub struct UnsupportedProtocol {
    pub server_max: u32,
}

/// The name a client announces in `SET_CLIENT_NAME`: non-empty, free of NUL
/// bytes, and short enough that the NUL-terminated packet fits the SDK limit.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct ClientName(String);

impl ClientName {
    pub fn new(name: impl Into<String>) -> Result<Self, ClientNameError> {
        let name = name.into();
        if name.is_empty() {
            return Err(ClientNameError::Empty);
        }
        if name.contains('\0') {
            return Err(ClientNameError::ContainsNul);
        }
        if name.len() >= super::wire::MAX_PACKET_SIZE as usize {
            return Err(ClientNameError::TooLong { len: name.len() });
        }
        Ok(Self(name))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for ClientName {
    type Error = ClientNameError;

    fn try_from(name: String) -> Result<Self, Self::Error> {
        Self::new(name)
    }
}

impl From<ClientName> for String {
    fn from(name: ClientName) -> Self {
        name.0
    }
}

impl fmt::Display for ClientName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ClientNameError {
    #[error("the OpenRGB client name is empty")]
    Empty,
    #[error("the OpenRGB client name contains a NUL byte")]
    ContainsNul,
    #[error("the OpenRGB client name is {len} bytes, too long for one SDK packet")]
    TooLong { len: usize },
}

/// Declares a `u32` bit-flag newtype with named constants.
macro_rules! wire_flags {
    ($(#[$meta:meta])* $name:ident { $($(#[$flag_meta:meta])* $flag:ident = $bit:expr;)* }) => {
        $(#[$meta])*
        #[derive(Clone, Copy, PartialEq, Eq, Hash, Default)]
        pub struct $name(u32);

        impl $name {
            $($(#[$flag_meta])* pub const $flag: Self = Self($bit);)*

            /// Every named flag, for `Debug`.
            const NAMED: &'static [(&'static str, Self)] = &[$((stringify!($flag), Self::$flag)),*];

            pub const fn from_bits(bits: u32) -> Self {
                Self(bits)
            }

            pub const fn bits(self) -> u32 {
                self.0
            }

            /// Every bit of `other` is set.
            pub const fn contains(self, other: Self) -> bool {
                self.0 & other.0 == other.0
            }

            /// Some bit of `other` is set.
            pub const fn intersects(self, other: Self) -> bool {
                self.0 & other.0 != 0
            }

            pub const fn union(self, other: Self) -> Self {
                Self(self.0 | other.0)
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                let mut rest = self.0;
                let mut first = true;
                write!(f, "{}(", stringify!($name))?;
                for (name, flag) in Self::NAMED {
                    if flag.0 != 0 && self.contains(*flag) {
                        if !first {
                            f.write_str(" | ")?;
                        }
                        f.write_str(name)?;
                        rest &= !flag.0;
                        first = false;
                    }
                }
                if rest != 0 {
                    if !first {
                        f.write_str(" | ")?;
                    }
                    write!(f, "{rest:#x}")?;
                }
                f.write_str(")")
            }
        }

        impl Serialize for $name {
            fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                serializer.serialize_u32(self.0)
            }
        }
    };
}

wire_flags! {
    /// `MODE_FLAG_*` (`RGBControllerInterface.h`).
    ModeFlags {
        HAS_SPEED = 1 << 0;
        HAS_DIRECTION_LR = 1 << 1;
        HAS_DIRECTION_UD = 1 << 2;
        HAS_DIRECTION_HV = 1 << 3;
        HAS_BRIGHTNESS = 1 << 4;
        HAS_PER_LED_COLOR = 1 << 5;
        HAS_MODE_SPECIFIC_COLOR = 1 << 6;
        HAS_RANDOM_COLOR = 1 << 7;
        MANUAL_SAVE = 1 << 8;
        AUTOMATIC_SAVE = 1 << 9;
        REQUIRES_ENTIRE_DEVICE = 1 << 10;
        /// Sent only at protocol 6; the server strips it below 6.
        HAS_DIRECTION_DIAG = 1 << 11;
    }
}

wire_flags! {
    /// `CONTROLLER_FLAG_*` (`RGBControllerInterface.h`).
    ControllerFlags {
        LOCAL = 1 << 0;
        REMOTE = 1 << 1;
        VIRTUAL = 1 << 2;
        HIDDEN = 1 << 3;
        RESET_BEFORE_UPDATE = 1 << 8;
        MANUALLY_CONFIGURABLE_NAME = 1 << 16;
        MANUALLY_CONFIGURABLE_DEVICE_SPECIFIC = 1 << 17;
        /// The controller's `display_name` holds a user-configured name.
        MANUALLY_CONFIGURED_NAME = 1 << 24;
        MANUALLY_CONFIGURED_DEVICE_SPECIFIC = 1 << 25;
    }
}

wire_flags! {
    /// `ZONE_FLAG_*` (`RGBControllerInterface.h`).
    ZoneFlags {
        MANUALLY_CONFIGURABLE_SIZE_EFFECTS_ONLY = 1 << 0;
        MANUALLY_CONFIGURABLE_SIZE = 1 << 1;
        MANUALLY_CONFIGURABLE_NAME = 1 << 2;
        MANUALLY_CONFIGURABLE_TYPE = 1 << 3;
        MANUALLY_CONFIGURABLE_MATRIX_MAP = 1 << 4;
        MANUALLY_CONFIGURABLE_SEGMENTS = 1 << 5;
        MANUALLY_CONFIGURABLE_DEVICE_SPECIFIC = 1 << 6;
        MANUALLY_CONFIGURED_SIZE = 1 << 12;
        MANUALLY_CONFIGURED_NAME = 1 << 13;
        MANUALLY_CONFIGURED_TYPE = 1 << 14;
        MANUALLY_CONFIGURED_MATRIX_MAP = 1 << 15;
        MANUALLY_CONFIGURED_SEGMENTS = 1 << 16;
        MANUALLY_CONFIGURED_DEVICE_SPECIFIC = 1 << 17;
        GEOMETRY_MAY_CHANGE = 1 << 24;
    }
}

wire_flags! {
    /// `SEGMENT_FLAG_*` (`RGBControllerInterface.h`).
    SegmentFlags {
        GROUP_START = 1 << 0;
        GROUP_MEMBER = 1 << 1;
    }
}

wire_flags! {
    /// `NET_CLIENT_FLAG_*` sent in `SET_CLIENT_FLAGS` (`NetworkProtocol.h`).
    ClientFlags {
        SUPPORTS_RGBCONTROLLER = 1 << 0;
        SUPPORTS_LOGMANAGER = 1 << 1;
        SUPPORTS_PROFILEMANAGER = 1 << 2;
        SUPPORTS_PLUGINMANAGER = 1 << 3;
        SUPPORTS_SETTINGSMANAGER = 1 << 4;
        REQUEST_LOCAL_CLIENT = 1 << 16;
    }
}

wire_flags! {
    /// `NET_SERVER_FLAG_*` received in `SET_SERVER_FLAGS` (`NetworkProtocol.h`).
    ServerFlags {
        SUPPORTS_RGBCONTROLLER = 1 << 0;
        SUPPORTS_LOGMANAGER = 1 << 1;
        SUPPORTS_PROFILEMANAGER = 1 << 2;
        SUPPORTS_PLUGINMANAGER = 1 << 3;
        SUPPORTS_SETTINGSMANAGER = 1 << 4;
        SUPPORTS_DETECTION = 1 << 5;
        SUPPORTS_DEVICE_INFO = 1 << 6;
        LOCAL_CLIENT = 1 << 16;
    }
}

/// A mode's colour selection, `MODE_COLORS_*` (`RGBControllerInterface.h`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ColorMode {
    /// 0: the mode shows no colours.
    None,
    /// 1: the controller's per-LED colours apply.
    PerLed,
    /// 2: the mode's own colour list applies.
    ModeSpecific,
    /// 3: the device picks colours itself.
    Random,
    /// Any other value, kept as received.
    Unknown(u32),
}

impl ColorMode {
    pub const fn from_wire(value: u32) -> Self {
        match value {
            0 => Self::None,
            1 => Self::PerLed,
            2 => Self::ModeSpecific,
            3 => Self::Random,
            other => Self::Unknown(other),
        }
    }

    pub const fn to_wire(self) -> u32 {
        match self {
            Self::None => 0,
            Self::PerLed => 1,
            Self::ModeSpecific => 2,
            Self::Random => 3,
            Self::Unknown(value) => value,
        }
    }
}

impl fmt::Display for ColorMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::None => f.write_str("none"),
            Self::PerLed => f.write_str("per-LED"),
            Self::ModeSpecific => f.write_str("mode-specific"),
            Self::Random => f.write_str("random"),
            Self::Unknown(value) => write!(f, "colour mode {value}"),
        }
    }
}

impl Serialize for ColorMode {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

/// A mode's direction value, `MODE_DIRECTION_*` (`RGBControllerInterface.h`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize)]
#[serde(transparent)]
pub struct Direction(pub u32);

impl Direction {
    pub const LEFT: Self = Self(0);
    pub const RIGHT: Self = Self(1);
    pub const UP: Self = Self(2);
    pub const DOWN: Self = Self(3);
    pub const HORIZONTAL: Self = Self(4);
    pub const VERTICAL: Self = Self(5);
    pub const UP_LEFT: Self = Self(6);
    pub const UP_RIGHT: Self = Self(7);
    pub const DOWN_LEFT: Self = Self(8);
    pub const DOWN_RIGHT: Self = Self(9);
}

impl fmt::Display for Direction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match *self {
            Self::LEFT => "left",
            Self::RIGHT => "right",
            Self::UP => "up",
            Self::DOWN => "down",
            Self::HORIZONTAL => "horizontal",
            Self::VERTICAL => "vertical",
            Self::UP_LEFT => "up-left",
            Self::UP_RIGHT => "up-right",
            Self::DOWN_LEFT => "down-left",
            Self::DOWN_RIGHT => "down-right",
            Self(other) => return write!(f, "direction {other}"),
        };
        f.write_str(name)
    }
}

/// A zone or segment type, `ZONE_TYPE_*` (`RGBControllerInterface.h`). Below
/// protocol 6 the server reports the loop and segmented types as their plain
/// linear or matrix forms.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ZoneType {
    Single,
    Linear,
    Matrix,
    LinearLoop,
    MatrixLoopX,
    MatrixLoopY,
    Segmented,
    Unknown(u32),
}

impl ZoneType {
    pub const fn from_wire(value: u32) -> Self {
        match value {
            0 => Self::Single,
            1 => Self::Linear,
            2 => Self::Matrix,
            3 => Self::LinearLoop,
            4 => Self::MatrixLoopX,
            5 => Self::MatrixLoopY,
            6 => Self::Segmented,
            other => Self::Unknown(other),
        }
    }

    pub const fn to_wire(self) -> u32 {
        match self {
            Self::Single => 0,
            Self::Linear => 1,
            Self::Matrix => 2,
            Self::LinearLoop => 3,
            Self::MatrixLoopX => 4,
            Self::MatrixLoopY => 5,
            Self::Segmented => 6,
            Self::Unknown(value) => value,
        }
    }
}

impl Serialize for ZoneType {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Unknown(value) => serializer.serialize_u32(*value),
            known => serializer.collect_str(&format_args!("{known:?}")),
        }
    }
}

/// A controller's device type, `DEVICE_TYPE_*` (`RGBControllerInterface.h`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(transparent)]
pub struct DeviceType(pub i32);

impl DeviceType {
    const NAMES: [&'static str; 22] = [
        "motherboard",
        "dram",
        "gpu",
        "cooler",
        "ledstrip",
        "keyboard",
        "mouse",
        "mousemat",
        "headset",
        "headset stand",
        "gamepad",
        "light",
        "speaker",
        "virtual",
        "storage",
        "case",
        "microphone",
        "accessory",
        "keypad",
        "laptop",
        "monitor",
        "unknown",
    ];

    /// The type's name, `None` for a value outside the enum.
    pub fn name(self) -> Option<&'static str> {
        usize::try_from(self.0)
            .ok()
            .and_then(|index| Self::NAMES.get(index).copied())
    }
}

/// `NET_PACKET_STATUS_*`, the status an ACK carries (`NetworkProtocol.h`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AckStatus {
    Ok,
    ErrorGeneric,
    ErrorUnsupported,
    ErrorNotAllowed,
    ErrorInvalidId,
    ErrorInvalidData,
    Unknown(u32),
}

impl AckStatus {
    pub const fn from_wire(value: u32) -> Self {
        match value {
            0 => Self::Ok,
            1 => Self::ErrorGeneric,
            2 => Self::ErrorUnsupported,
            3 => Self::ErrorNotAllowed,
            4 => Self::ErrorInvalidId,
            5 => Self::ErrorInvalidData,
            other => Self::Unknown(other),
        }
    }

    pub const fn to_wire(self) -> u32 {
        match self {
            Self::Ok => 0,
            Self::ErrorGeneric => 1,
            Self::ErrorUnsupported => 2,
            Self::ErrorNotAllowed => 3,
            Self::ErrorInvalidId => 4,
            Self::ErrorInvalidData => 5,
            Self::Unknown(value) => value,
        }
    }
}

impl fmt::Display for AckStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ok => f.write_str("OK"),
            Self::ErrorGeneric => f.write_str("ERROR_GENERIC"),
            Self::ErrorUnsupported => f.write_str("ERROR_UNSUPPORTED"),
            Self::ErrorNotAllowed => f.write_str("ERROR_NOT_ALLOWED"),
            Self::ErrorInvalidId => f.write_str("ERROR_INVALID_ID"),
            Self::ErrorInvalidData => f.write_str("ERROR_INVALID_DATA"),
            Self::Unknown(value) => write!(f, "status {value}"),
        }
    }
}

impl Serialize for AckStatus {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

/// `RGBCONTROLLER_UPDATE_REASON_*`, why a `SIGNALUPDATE` was sent
/// (`RGBControllerInterface.h`). The reason selects the payload: `UpdateLeds`
/// carries only the colours, every other reason the whole description.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UpdateReason {
    UpdateLeds,
    UpdateMode,
    SaveMode,
    ConfigureZone,
    ClearSegments,
    AddSegment,
    Hidden,
    Unhidden,
    SetDeviceSpecificConfiguration,
    SetDeviceSpecificZoneConfiguration,
    ConfigureDevice,
    DeviceChanged,
    Unknown(u32),
}

impl UpdateReason {
    pub const fn from_wire(value: u32) -> Self {
        match value {
            0 => Self::UpdateLeds,
            1 => Self::UpdateMode,
            2 => Self::SaveMode,
            3 => Self::ConfigureZone,
            4 => Self::ClearSegments,
            5 => Self::AddSegment,
            6 => Self::Hidden,
            7 => Self::Unhidden,
            8 => Self::SetDeviceSpecificConfiguration,
            9 => Self::SetDeviceSpecificZoneConfiguration,
            10 => Self::ConfigureDevice,
            11 => Self::DeviceChanged,
            other => Self::Unknown(other),
        }
    }

    pub const fn to_wire(self) -> u32 {
        match self {
            Self::UpdateLeds => 0,
            Self::UpdateMode => 1,
            Self::SaveMode => 2,
            Self::ConfigureZone => 3,
            Self::ClearSegments => 4,
            Self::AddSegment => 5,
            Self::Hidden => 6,
            Self::Unhidden => 7,
            Self::SetDeviceSpecificConfiguration => 8,
            Self::SetDeviceSpecificZoneConfiguration => 9,
            Self::ConfigureDevice => 10,
            Self::DeviceChanged => 11,
            Self::Unknown(value) => value,
        }
    }
}

/// One entry of a controller's (or, at protocol 6, a zone's) mode list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ModeDescription {
    pub name: WireString,
    /// The device-specific mode value; carried below protocol 6 only, where the
    /// client must echo it back in `UPDATEMODE`.
    pub value: Option<i32>,
    pub flags: ModeFlags,
    pub speed_min: u32,
    pub speed_max: u32,
    pub brightness_min: u32,
    pub brightness_max: u32,
    pub colors_min: u32,
    pub colors_max: u32,
    pub speed: u32,
    pub brightness: u32,
    pub direction: Direction,
    pub color_mode: ColorMode,
    pub colors: Vec<RgbColor>,
}

/// A zone's or segment's LED layout grid.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MatrixMap {
    pub height: u32,
    pub width: u32,
    /// `height * width` LED indices, row by row.
    pub map: Vec<u32>,
}

/// Protocol 6 additions to a segment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SegmentV6 {
    pub matrix: Option<MatrixMap>,
    pub flags: SegmentFlags,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SegmentDescription {
    pub name: WireString,
    pub segment_type: ZoneType,
    pub start_idx: u32,
    pub leds_count: u32,
    pub v6: Option<SegmentV6>,
}

/// Protocol 6 additions to a zone: its own modes and display name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ZoneV6 {
    /// The zone's active per-zone mode, `-1` when the device mode applies.
    pub active_mode: i32,
    pub modes: Vec<ModeDescription>,
    pub display_name: WireString,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ZoneDescription {
    pub name: WireString,
    pub zone_type: ZoneType,
    pub leds_min: u32,
    pub leds_max: u32,
    pub leds_count: u32,
    pub matrix: Option<MatrixMap>,
    pub segments: Vec<SegmentDescription>,
    pub flags: ZoneFlags,
    pub v6: Option<ZoneV6>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LedDescription {
    pub name: WireString,
    /// The device-specific LED value; carried below protocol 6 only.
    pub value: Option<u32>,
}

/// Protocol 6 additions to a controller.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ControllerV6 {
    /// A user-configured name, meaningful only while
    /// [`ControllerFlags::MANUALLY_CONFIGURED_NAME`] is set.
    pub display_name: WireString,
    /// Device-specific configuration JSON.
    pub configuration: WireString,
}

/// A controller as `REQUEST_CONTROLLER_DATA` describes it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ControllerDescription {
    pub device_type: DeviceType,
    pub name: WireString,
    pub vendor: WireString,
    pub description: WireString,
    pub version: WireString,
    pub serial: WireString,
    pub location: WireString,
    pub active_mode: i32,
    pub modes: Vec<ModeDescription>,
    pub zones: Vec<ZoneDescription>,
    pub leds: Vec<LedDescription>,
    /// One colour per LED, the values `UPDATELEDS` replaces.
    pub colors: Vec<RgbColor>,
    pub led_display_names: Vec<WireString>,
    pub flags: ControllerFlags,
    pub v6: Option<ControllerV6>,
}

impl ControllerDescription {
    /// The name OpenRGB shows for this controller: the user-configured
    /// `display_name` when [`ControllerFlags::MANUALLY_CONFIGURED_NAME`] is set,
    /// otherwise `name` (`RGBController::GetDisplayName`). Below protocol 6 the
    /// block has no `display_name`, so the name is always `name` there.
    pub fn display_name(&self) -> &WireString {
        match &self.v6 {
            Some(v6)
                if self
                    .flags
                    .contains(ControllerFlags::MANUALLY_CONFIGURED_NAME) =>
            {
                &v6.display_name
            }
            _ => &self.name,
        }
    }

    /// The mode at `active_mode`, when it indexes the mode list.
    pub fn active_mode(&self) -> Option<&ModeDescription> {
        usize::try_from(self.active_mode)
            .ok()
            .and_then(|index| self.modes.get(index))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn controller(
        name: &str,
        display_name: Option<&str>,
        flags: ControllerFlags,
    ) -> ControllerDescription {
        ControllerDescription {
            device_type: DeviceType(1),
            name: name.into(),
            vendor: "ENE".into(),
            description: WireString::default(),
            version: WireString::default(),
            serial: WireString::default(),
            location: WireString::default(),
            active_mode: 0,
            modes: Vec::new(),
            zones: Vec::new(),
            leds: Vec::new(),
            colors: Vec::new(),
            led_display_names: Vec::new(),
            flags,
            v6: display_name.map(|display_name| ControllerV6 {
                display_name: display_name.into(),
                configuration: WireString::default(),
            }),
        }
    }

    #[test]
    fn display_name_is_the_configured_name_only_with_the_flag() {
        let plain = controller("ENE DRAM", Some("Left stick"), ControllerFlags::LOCAL);
        assert_eq!(plain.display_name(), &WireString::from("ENE DRAM"));

        let configured = controller(
            "ENE DRAM",
            Some("Left stick"),
            ControllerFlags::LOCAL.union(ControllerFlags::MANUALLY_CONFIGURED_NAME),
        );
        assert_eq!(configured.display_name(), &WireString::from("Left stick"));

        let unset_display = controller("ENE DRAM", Some(""), ControllerFlags::default());
        assert_eq!(unset_display.display_name(), &WireString::from("ENE DRAM"));

        let v5 = controller("ENE DRAM", None, ControllerFlags::MANUALLY_CONFIGURED_NAME);
        assert_eq!(v5.display_name(), &WireString::from("ENE DRAM"));
    }

    #[test]
    fn active_mode_is_the_mode_at_the_index_when_it_exists() {
        let mut dram = controller("ENE DRAM", None, ControllerFlags::LOCAL);
        dram.modes = vec![
            ModeDescription {
                name: "Direct".into(),
                value: None,
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
                colors: Vec::new(),
            };
            2
        ];
        dram.modes[1].name = "Static".into();
        dram.active_mode = 1;
        assert_eq!(
            dram.active_mode().map(|mode| mode.name.to_string()),
            Some("Static".into())
        );
        dram.active_mode = 2;
        assert_eq!(dram.active_mode(), None);
        dram.active_mode = -1;
        assert_eq!(dram.active_mode(), None);
    }

    #[test]
    fn negotiation_takes_the_lower_version_and_rejects_below_five() {
        assert_eq!(
            ProtocolVersion::negotiate(6, ProtocolVersion::V6),
            Ok(ProtocolVersion::V6)
        );
        assert_eq!(
            ProtocolVersion::negotiate(7, ProtocolVersion::V6),
            Ok(ProtocolVersion::V6)
        );
        assert_eq!(
            ProtocolVersion::negotiate(6, ProtocolVersion::V5),
            Ok(ProtocolVersion::V5)
        );
        assert_eq!(
            ProtocolVersion::negotiate(5, ProtocolVersion::V6),
            Ok(ProtocolVersion::V5)
        );
        assert_eq!(
            ProtocolVersion::negotiate(4, ProtocolVersion::V6),
            Err(UnsupportedProtocol { server_max: 4 })
        );
        assert_eq!(
            ProtocolVersion::negotiate(0, ProtocolVersion::V5),
            Err(UnsupportedProtocol { server_max: 0 })
        );
    }

    #[test]
    fn protocol_version_serde_is_the_number() {
        assert_eq!(serde_json::to_string(&ProtocolVersion::V6).unwrap(), "6");
        assert_eq!(
            serde_json::from_str::<ProtocolVersion>("5").unwrap(),
            ProtocolVersion::V5
        );
        assert!(serde_json::from_str::<ProtocolVersion>("4").is_err());
    }

    #[test]
    fn enums_round_trip_their_wire_values() {
        for value in 0..16 {
            assert_eq!(ColorMode::from_wire(value).to_wire(), value);
            assert_eq!(ZoneType::from_wire(value).to_wire(), value);
            assert_eq!(AckStatus::from_wire(value).to_wire(), value);
            assert_eq!(UpdateReason::from_wire(value).to_wire(), value);
        }
        assert_eq!(AckStatus::from_wire(4), AckStatus::ErrorInvalidId);
        assert_eq!(UpdateReason::from_wire(11), UpdateReason::DeviceChanged);
        assert_eq!(ColorMode::from_wire(2), ColorMode::ModeSpecific);
    }

    #[test]
    fn flag_values_match_the_headers() {
        assert_eq!(ControllerFlags::MANUALLY_CONFIGURED_NAME.bits(), 1 << 24);
        assert_eq!(ModeFlags::HAS_PER_LED_COLOR.bits(), 32);
        assert_eq!(ModeFlags::HAS_MODE_SPECIFIC_COLOR.bits(), 64);
        assert_eq!(ModeFlags::HAS_DIRECTION_DIAG.bits(), 2048);
        assert_eq!(ClientFlags::SUPPORTS_RGBCONTROLLER.bits(), 1);
        assert_eq!(
            format!("{:?}", ModeFlags::from_bits(0x21 | 0x8000)),
            "ModeFlags(HAS_SPEED | HAS_PER_LED_COLOR | 0x8000)"
        );
    }

    /// Bits round-trip, set operations, `Debug` naming and serialisation for a
    /// flag type, using two of its flags.
    macro_rules! check_flags {
        ($($flags:ident: $first:ident, $second:ident;)*) => {$(
            let both = $flags::$first.union($flags::$second);
            assert_eq!($flags::from_bits(both.bits()), both);
            assert!(both.contains($flags::$first) && both.contains($flags::$second));
            assert!(!$flags::$first.contains(both));
            assert!(both.intersects($flags::$second));
            assert!(!$flags::$first.intersects($flags::$second));
            assert_eq!(
                format!("{both:?}"),
                format!("{}({} | {})", stringify!($flags), stringify!($first), stringify!($second))
            );
            assert_eq!(serde_json::to_string(&both).unwrap(), both.bits().to_string());
        )*};
    }

    #[test]
    fn flag_types_behave_as_bit_sets() {
        check_flags! {
            ModeFlags: HAS_SPEED, HAS_DIRECTION_DIAG;
            ControllerFlags: LOCAL, MANUALLY_CONFIGURED_NAME;
            ZoneFlags: MANUALLY_CONFIGURABLE_SIZE_EFFECTS_ONLY, GEOMETRY_MAY_CHANGE;
            SegmentFlags: GROUP_START, GROUP_MEMBER;
            ClientFlags: SUPPORTS_RGBCONTROLLER, REQUEST_LOCAL_CLIENT;
            ServerFlags: SUPPORTS_RGBCONTROLLER, LOCAL_CLIENT;
        }
    }

    #[test]
    fn client_name_validation() {
        assert_eq!(ClientName::new("vogix").unwrap().as_str(), "vogix");
        assert_eq!(ClientName::new(""), Err(ClientNameError::Empty));
        assert_eq!(
            ClientName::new("vo\0gix"),
            Err(ClientNameError::ContainsNul)
        );
        assert!(serde_json::from_str::<ClientName>("\"\"").is_err());
        assert_eq!(
            serde_json::from_str::<ClientName>("\"vogix\"")
                .unwrap()
                .as_str(),
            "vogix"
        );
    }

    #[test]
    fn device_type_names() {
        assert_eq!(DeviceType(1).name(), Some("dram"));
        assert_eq!(DeviceType(21).name(), Some("unknown"));
        assert_eq!(DeviceType(22).name(), None);
        assert_eq!(DeviceType(-1).name(), None);
    }
}

//! Validated value types shared by `/etc/vogix/machine.json` and the published
//! `palette.json`.
//!
//! Every one of them parses from exactly the text the Nix module or the
//! publisher writes, and nothing else, so a value that reaches an owner has
//! already been checked: slot and device names are identifiers, colours are
//! `#rrggbb`, USB ids are four lowercase hex digits.

use pr4xis_domains::natural::colors::Rgb as PraxisRgb;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::str::FromStr;

/// The `schema` field of both machine files. Only `1` parses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SchemaV1;

impl Serialize for SchemaV1 {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_u32(1)
    }
}

impl<'de> Deserialize<'de> for SchemaV1 {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        match u64::deserialize(deserializer)? {
            1 => Ok(SchemaV1),
            other => Err(serde::de::Error::custom(format!(
                "schema {other} is not supported: this vogix reads schema 1"
            ))),
        }
    }
}

/// A string newtype whose only constructor is its validator; serde goes
/// through the same validator (`try_from = "String"`).
macro_rules! validated_string {
    ($(#[$meta:meta])* $name:ident, $check:path) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[serde(try_from = "String", into = "String")]
        pub struct $name(String);

        impl $name {
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl TryFrom<String> for $name {
            type Error = String;
            fn try_from(value: String) -> Result<Self, String> {
                $check(&value).map_err(|why| {
                    format!("invalid {} {:?}: {why}", stringify!($name), value)
                })?;
                Ok(Self(value))
            }
        }

        impl FromStr for $name {
            type Err = String;
            fn from_str(value: &str) -> Result<Self, String> {
                Self::try_from(value.to_string())
            }
        }

        impl From<$name> for String {
            fn from(value: $name) -> String {
                value.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }
    };
}

/// `^[A-Za-z0-9_-]{1,64}$` — the charset `vogix.hardware.devices.<name>.slot`
/// accepts, and the one every theme loader's keys use (`base0D`,
/// `foreground_text`, `color04`).
fn check_identifier(value: &str) -> Result<(), &'static str> {
    if value.is_empty() {
        return Err("it is empty");
    }
    if value.len() > 64 {
        return Err("it is longer than 64 bytes");
    }
    if !value
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
    {
        return Err("only A-Z, a-z, 0-9, '_' and '-' are allowed");
    }
    Ok(())
}

/// A POSIX-portable user name as NixOS accepts it: 1–32 bytes of
/// `[A-Za-z0-9._-]`, not starting with '-', optionally ending in '$'.
fn check_user_name(value: &str) -> Result<(), &'static str> {
    let body = value.strip_suffix('$').unwrap_or(value);
    if body.is_empty() || value.len() > 32 {
        return Err("a user name is 1 to 32 bytes");
    }
    if body.starts_with('-') {
        return Err("a user name does not start with '-'");
    }
    if !body
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
    {
        return Err("only A-Z, a-z, 0-9, '.', '_' and '-' are allowed");
    }
    Ok(())
}

/// Free text that ends up in logs, `STATUS=` lines and OpenRGB SDK strings:
/// 1–128 bytes with no control characters (so no NUL and no newline).
fn check_label(value: &str) -> Result<(), &'static str> {
    if value.is_empty() {
        return Err("it is empty");
    }
    if value.len() > 128 {
        return Err("it is longer than 128 bytes");
    }
    if value.chars().any(char::is_control) {
        return Err("it contains a control character");
    }
    Ok(())
}

validated_string!(
    /// A palette slot name (`base01`, `active`, `color04`).
    SlotName,
    check_identifier
);

validated_string!(
    /// The name of a declared machine device (`dram-rgb`, `kraken-ring`): the
    /// key of `vogix.hardware.devices`, and the name every log and status
    /// line uses for it.
    DeviceName,
    check_identifier
);

validated_string!(
    /// The machine owner's user name.
    UserName,
    check_user_name
);

validated_string!(
    /// Free text: a theme or variant name, an OpenRGB device-name substring,
    /// a mode name, the SDK client name.
    Label,
    check_label
);

/// A USB vendor or product id, written as exactly four lowercase hex digits
/// (`1e71`), the form `vogix.hardware.devices.<name>.provider.command.hotplug`
/// declares.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct UsbId(u16);

impl UsbId {
    pub const fn new(value: u16) -> Self {
        Self(value)
    }

    pub const fn value(self) -> u16 {
        self.0
    }
}

impl FromStr for UsbId {
    type Err = String;
    fn from_str(value: &str) -> Result<Self, String> {
        if value.len() != 4
            || !value
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(format!(
                "invalid USB id {value:?}: expected four lowercase hex digits"
            ));
        }
        u16::from_str_radix(value, 16)
            .map(Self)
            .map_err(|e| format!("invalid USB id {value:?}: {e}"))
    }
}

impl TryFrom<String> for UsbId {
    type Error = String;
    fn try_from(value: String) -> Result<Self, String> {
        value.parse()
    }
}

impl From<UsbId> for String {
    fn from(value: UsbId) -> String {
        value.to_string()
    }
}

impl fmt::Display for UsbId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04x}", self.0)
    }
}

/// An sRGB colour, written `#rrggbb`. Parsing accepts exactly '#' and six hex
/// digits (either case, as theme files write them); it always serializes in
/// lowercase, so equal colours give equal bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Rgb(PraxisRgb);

impl Rgb {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self(PraxisRgb { r, g, b })
    }

    pub const fn channels(self) -> (u8, u8, u8) {
        (self.0.r, self.0.g, self.0.b)
    }

    /// The colour as the praxis colour-science value.
    pub const fn to_praxis(self) -> PraxisRgb {
        self.0
    }

    /// `rrggbb` without the '#': the form a command device's `{{color}}`
    /// argument takes.
    pub fn bare_hex(self) -> String {
        format!("{:02x}{:02x}{:02x}", self.0.r, self.0.g, self.0.b)
    }
}

impl FromStr for Rgb {
    type Err = String;
    fn from_str(value: &str) -> Result<Self, String> {
        let invalid = || format!("invalid colour {value:?}: expected #rrggbb");
        let hex = value.strip_prefix('#').ok_or_else(invalid)?;
        if hex.len() != 6 || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(invalid());
        }
        // All six bytes are ASCII hex digits, so every slice is a char
        // boundary and every parse succeeds.
        let channel = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).map_err(|_| invalid());
        Ok(Self::new(channel(0)?, channel(2)?, channel(4)?))
    }
}

impl TryFrom<String> for Rgb {
    type Error = String;
    fn try_from(value: String) -> Result<Self, String> {
        value.parse()
    }
}

impl From<Rgb> for String {
    fn from(value: Rgb) -> String {
        value.to_string()
    }
}

impl fmt::Display for Rgb {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{}", self.bare_hex())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn schema_accepts_only_one() {
        assert_eq!(serde_json::from_str::<SchemaV1>("1").unwrap(), SchemaV1);
        let err = serde_json::from_str::<SchemaV1>("2").unwrap_err();
        assert!(err.to_string().contains("schema 2 is not supported"));
        assert!(serde_json::from_str::<SchemaV1>("\"1\"").is_err());
        assert_eq!(serde_json::to_string(&SchemaV1).unwrap(), "1");
    }

    #[test]
    fn slot_names_are_identifiers() {
        for ok in ["base01", "base0D", "foreground_text", "color-04", "a"] {
            assert!(ok.parse::<SlotName>().is_ok(), "{ok}");
        }
        for bad in ["", "base 01", "base01\n", "{{base01}}", "#base01", "ü"] {
            assert!(bad.parse::<SlotName>().is_err(), "{bad:?}");
        }
        assert!("a".repeat(64).parse::<SlotName>().is_ok());
        assert!("a".repeat(65).parse::<SlotName>().is_err());
    }

    #[test]
    fn user_names_follow_the_portable_charset() {
        for ok in ["logger", "t", "vogix", "svc$", "a.b_c-d"] {
            assert!(ok.parse::<UserName>().is_ok(), "{ok}");
        }
        for bad in ["", "-x", "a b", "$", "a/b", &"a".repeat(33)] {
            assert!(bad.parse::<UserName>().is_err(), "{bad:?}");
        }
    }

    #[test]
    fn labels_reject_control_characters() {
        assert!("ENE DRAM".parse::<Label>().is_ok());
        assert!("NZXT Kraken 2024 ELITE Series RGB".parse::<Label>().is_ok());
        for bad in ["", "a\nb", "a\0b", "tab\there"] {
            assert!(bad.parse::<Label>().is_err(), "{bad:?}");
        }
    }

    #[test]
    fn usb_ids_are_four_lowercase_hex_digits() {
        assert_eq!("1e71".parse::<UsbId>().unwrap(), UsbId::new(0x1e71));
        assert_eq!("0627".parse::<UsbId>().unwrap().to_string(), "0627");
        for bad in ["1E71", "1e7", "01e71", "+e71", "1e7g", "", "0x1e"] {
            assert!(bad.parse::<UsbId>().is_err(), "{bad:?}");
        }
        assert_eq!(
            serde_json::to_string(&UsbId::new(0x3012)).unwrap(),
            "\"3012\""
        );
    }

    #[test]
    fn colours_parse_exactly_hash_and_six_hex_digits() {
        assert_eq!(
            "#2E3440".parse::<Rgb>().unwrap(),
            Rgb::new(0x2e, 0x34, 0x40)
        );
        assert_eq!(Rgb::new(0x2e, 0x34, 0x40).to_string(), "#2e3440");
        assert_eq!(Rgb::new(0x2e, 0x34, 0x40).bare_hex(), "2e3440");
        for bad in [
            "2e3440", "#2e344", "#2e34400", "#+e3440", "#2e34 0", "#ééé", "", "#",
        ] {
            assert!(bad.parse::<Rgb>().is_err(), "{bad:?}");
        }
    }

    proptest! {
        #[test]
        fn colours_round_trip_through_their_text(r: u8, g: u8, b: u8) {
            let c = Rgb::new(r, g, b);
            let json = serde_json::to_string(&c).unwrap();
            prop_assert_eq!(serde_json::from_str::<Rgb>(&json).unwrap(), c);
            prop_assert_eq!(c.to_string().to_lowercase(), c.to_string());
        }

        #[test]
        fn colour_parsing_never_panics(s in "\\PC{0,10}") {
            let _ = s.parse::<Rgb>();
        }
    }
}

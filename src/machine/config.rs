//! `/etc/vogix/machine.json`: what the machine owners own.
//!
//! The NixOS module renders it with `builtins.toJSON`; this is its typed
//! reader. Unknown fields are errors, `provider` is serde's externally
//! tagged enum (the JSON shape of `types.attrTag`), and every value is a
//! validated type, so no device name, slot or command is interpreted here
//! beyond what the declaration says.

use super::types::{DeviceName, Label, SchemaV1, SlotName, UsbId, UserName};
use crate::fsutil;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;
use std::fs::File;
use std::io;
use std::net::Ipv4Addr;
use std::path::{Component, Path, PathBuf};

/// Where the NixOS module installs the machine config.
pub const MACHINE_CONFIG_PATH: &str = "/etc/vogix/machine.json";

/// Upper bound for either machine file; the rendered files are a few KiB.
pub const MAX_MACHINE_FILE_BYTES: u64 = 64 * 1024;

/// The exact argv element a command device replaces with the colour.
pub const COLOR_PLACEHOLDER: &str = "{{color}}";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct MachineConfig {
    pub schema: SchemaV1,
    /// The user whose published palette the machine surfaces follow.
    pub owner: UserName,
    /// The directory the owner publishes `palette.json` into.
    pub drop_zone: DropZone,
    pub console: ConsoleConfig,
    /// The OpenRGB SDK server; present when any device uses the openrgb provider.
    pub openrgb: Option<OpenRgbEndpoint>,
    pub devices: BTreeMap<DeviceName, DeviceSpec>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConsoleConfig {
    /// Whether the machine owner writes the kernel's VT palette.
    pub enable: bool,
}

/// An absolute, normalized directory path (no `.` or `..` components).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "PathBuf", into = "PathBuf")]
pub struct DropZone(PathBuf);

impl DropZone {
    pub fn as_path(&self) -> &Path {
        &self.0
    }
}

impl TryFrom<PathBuf> for DropZone {
    type Error = String;
    fn try_from(path: PathBuf) -> Result<Self, String> {
        let normalized = path.is_absolute()
            && path
                .components()
                .all(|c| matches!(c, Component::RootDir | Component::Normal(_)));
        if !normalized || path.parent().is_none() {
            return Err(format!(
                "invalid drop zone {}: expected an absolute path without '.' or '..' below /",
                path.display()
            ));
        }
        Ok(Self(path))
    }
}

impl From<DropZone> for PathBuf {
    fn from(zone: DropZone) -> PathBuf {
        zone.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct OpenRgbEndpoint {
    /// A loopback address: the owner's connect is blocking and must either
    /// complete or be refused at once.
    pub host: Ipv4Addr,
    pub port: u16,
    pub max_protocol: MaxProtocol,
    pub client_name: Label,
}

/// The highest OpenRGB SDK protocol version the client offers
/// (`vogix.openrgb.client.maxProtocol`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "u32", into = "u32")]
pub enum MaxProtocol {
    V5,
    V6,
}

impl MaxProtocol {
    pub const fn number(self) -> u32 {
        match self {
            Self::V5 => 5,
            Self::V6 => 6,
        }
    }
}

impl TryFrom<u32> for MaxProtocol {
    type Error = String;
    fn try_from(value: u32) -> Result<Self, String> {
        match value {
            5 => Ok(Self::V5),
            6 => Ok(Self::V6),
            other => Err(format!(
                "maxProtocol {other} is not supported: vogix speaks OpenRGB SDK protocol 5 or 6"
            )),
        }
    }
}

impl From<MaxProtocol> for u32 {
    fn from(value: MaxProtocol) -> u32 {
        value.number()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeviceSpec {
    /// The palette slot whose colour the device shows.
    pub slot: SlotName,
    pub provider: Provider,
}

/// How a device is driven. Externally tagged: `{"openrgb": {...}}` or
/// `{"command": {...}}`, exactly one key — the JSON of `types.attrTag`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    Openrgb(OpenRgbDevice),
    Command(CommandDevice),
}

/// Every OpenRGB controller whose display name contains `name_contains`
/// (ASCII case-insensitive) is set to the mode named `mode`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct OpenRgbDevice {
    pub name_contains: Label,
    pub mode: Label,
}

/// A command run with the colour as an argument.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommandDevice {
    pub argv: Argv,
    #[serde(default)]
    pub hotplug: Hotplug,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Hotplug {
    /// Re-run the command when a hidraw node of this USB device appears.
    pub hidraw: Option<HidrawMatch>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct HidrawMatch {
    pub vendor_id: UsbId,
    pub product_id: UsbId,
}

impl fmt::Display for HidrawMatch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.vendor_id, self.product_id)
    }
}

/// A command line: non-empty, `argv[0]` an absolute path, no NUL bytes, and
/// the colour placeholder only ever a whole argument (an argument that merely
/// contains it would reach the command literally).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "Vec<String>", into = "Vec<String>")]
pub struct Argv(Vec<String>);

impl Argv {
    pub fn program(&self) -> &str {
        &self.0[0]
    }

    pub fn args(&self) -> &[String] {
        &self.0[1..]
    }

    /// The argv with every [`COLOR_PLACEHOLDER`] element replaced by `rrggbb`.
    pub fn render(&self, color: super::types::Rgb) -> Vec<String> {
        self.0
            .iter()
            .map(|arg| {
                if arg == COLOR_PLACEHOLDER {
                    color.bare_hex()
                } else {
                    arg.clone()
                }
            })
            .collect()
    }
}

impl TryFrom<Vec<String>> for Argv {
    type Error = String;
    fn try_from(argv: Vec<String>) -> Result<Self, String> {
        let Some(program) = argv.first() else {
            return Err("argv is empty".into());
        };
        if !Path::new(program).is_absolute() {
            return Err(format!("argv[0] {program:?} is not an absolute path"));
        }
        for arg in &argv {
            if arg.contains('\0') {
                return Err(format!("argv element {arg:?} contains a NUL byte"));
            }
            if arg != COLOR_PLACEHOLDER && arg.contains(COLOR_PLACEHOLDER) {
                return Err(format!(
                    "argv element {arg:?} contains {COLOR_PLACEHOLDER} but is not exactly \
                     {COLOR_PLACEHOLDER}; the placeholder must be a whole argument"
                ));
            }
        }
        Ok(Self(argv))
    }
}

impl From<Argv> for Vec<String> {
    fn from(argv: Argv) -> Vec<String> {
        argv.0
    }
}

/// Why `/etc/vogix/machine.json` (or a file given to `vogix machine
/// validate`) was not accepted.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("{path} does not exist")]
    Absent { path: PathBuf },
    #[error("cannot read {path}: {source}")]
    Io { path: PathBuf, source: io::Error },
    #[error("{path} is not a regular file")]
    NotRegularFile { path: PathBuf },
    #[error("{path} is {size} bytes; the limit is {max}")]
    TooLarge { path: PathBuf, size: u64, max: u64 },
    #[error("{path}: {source}")]
    Invalid {
        path: PathBuf,
        source: serde_json::Error,
    },
    #[error("{path}: {reason}")]
    Inconsistent { path: PathBuf, reason: String },
}

impl MachineConfig {
    /// Read and check a machine config. The file may be a symlink (NixOS
    /// installs `/etc` entries as links into the store); it must resolve to a
    /// regular file of at most [`MAX_MACHINE_FILE_BYTES`].
    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        let io_err = |source: io::Error| match source.kind() {
            io::ErrorKind::NotFound => ConfigError::Absent {
                path: path.to_path_buf(),
            },
            _ => ConfigError::Io {
                path: path.to_path_buf(),
                source,
            },
        };
        let mut file = File::open(path).map_err(io_err)?;
        let meta = file.metadata().map_err(io_err)?;
        if !meta.is_file() {
            return Err(ConfigError::NotRegularFile {
                path: path.to_path_buf(),
            });
        }
        if meta.len() > MAX_MACHINE_FILE_BYTES {
            return Err(ConfigError::TooLarge {
                path: path.to_path_buf(),
                size: meta.len(),
                max: MAX_MACHINE_FILE_BYTES,
            });
        }
        let bytes = fsutil::read_capped(&mut file, MAX_MACHINE_FILE_BYTES).map_err(io_err)?;
        Self::from_json(&bytes).map_err(|e| e.at(path))
    }

    /// Parse and check a machine config from its JSON bytes.
    pub fn from_json(bytes: &[u8]) -> Result<Self, ParseError> {
        let config: Self = serde_json::from_slice(bytes).map_err(ParseError::Invalid)?;
        config.check().map_err(ParseError::Inconsistent)?;
        Ok(config)
    }

    /// The cross-field rules serde cannot state per field.
    fn check(&self) -> Result<(), String> {
        if let Some(endpoint) = &self.openrgb {
            if !endpoint.host.is_loopback() {
                return Err(format!(
                    "openrgb.host {} is not a loopback address",
                    endpoint.host
                ));
            }
            if endpoint.port == 0 {
                return Err("openrgb.port is 0".into());
            }
        }
        let openrgb_devices: Vec<&DeviceName> = self
            .devices
            .iter()
            .filter(|(_, spec)| matches!(spec.provider, Provider::Openrgb(_)))
            .map(|(name, _)| name)
            .collect();
        if self.openrgb.is_none() && !openrgb_devices.is_empty() {
            return Err(format!(
                "devices {} use the openrgb provider but openrgb is null",
                openrgb_devices
                    .iter()
                    .map(|n| n.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        Ok(())
    }

    /// The devices driven by the OpenRGB owner, in name order.
    pub fn openrgb_devices(
        &self,
    ) -> impl Iterator<Item = (&DeviceName, &SlotName, &OpenRgbDevice)> {
        self.devices
            .iter()
            .filter_map(|(name, spec)| match &spec.provider {
                Provider::Openrgb(device) => Some((name, &spec.slot, device)),
                Provider::Command(_) => None,
            })
    }

    /// The devices driven as commands by the local owner, in name order.
    pub fn command_devices(
        &self,
    ) -> impl Iterator<Item = (&DeviceName, &SlotName, &CommandDevice)> {
        self.devices
            .iter()
            .filter_map(|(name, spec)| match &spec.provider {
                Provider::Command(device) => Some((name, &spec.slot, device)),
                Provider::Openrgb(_) => None,
            })
    }
}

/// A machine config that did not parse or did not hold together, before a
/// path is attached.
#[derive(Debug)]
pub enum ParseError {
    Invalid(serde_json::Error),
    Inconsistent(String),
}

impl ParseError {
    fn at(self, path: &Path) -> ConfigError {
        let path = path.to_path_buf();
        match self {
            Self::Invalid(source) => ConfigError::Invalid { path, source },
            Self::Inconsistent(reason) => ConfigError::Inconsistent { path, reason },
        }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invalid(e) => write!(f, "{e}"),
            Self::Inconsistent(reason) => f.write_str(reason),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::machine::types::Rgb;
    use serde_json::json;
    use std::os::unix::fs::symlink;

    /// The shape the NixOS module renders for dram-rgb, keychron-k2-he and
    /// kraken-elite.
    fn rendered() -> serde_json::Value {
        json!({
            "schema": 1,
            "owner": "t",
            "dropZone": "/var/lib/vogix/machine",
            "console": { "enable": true },
            "openrgb": {
                "host": "127.0.0.1",
                "port": 6742,
                "maxProtocol": 6,
                "clientName": "vogix"
            },
            "devices": {
                "dram-rgb": {
                    "slot": "base01",
                    "provider": { "openrgb": { "nameContains": "ENE DRAM", "mode": "Static" } }
                },
                "keychron-k2-he": {
                    "slot": "base01",
                    "provider": { "openrgb": { "nameContains": "Keychron K2 HE", "mode": "Static" } }
                },
                "kraken-ring": {
                    "slot": "base01",
                    "provider": { "command": {
                        "argv": [
                            "/nix/store/0000000000000000000000000000000-liquidctl/bin/liquidctl",
                            "--match", "kraken", "set", "ring", "color", "fixed", "{{color}}"
                        ],
                        "hotplug": { "hidraw": { "vendorId": "1e71", "productId": "3012" } }
                    } }
                }
            }
        })
    }

    fn parse(value: &serde_json::Value) -> Result<MachineConfig, ParseError> {
        MachineConfig::from_json(value.to_string().as_bytes())
    }

    fn rejects(value: serde_json::Value, needle: &str) {
        let err = parse(&value).expect_err("must be rejected").to_string();
        assert!(err.contains(needle), "{err:?} does not mention {needle:?}");
    }

    #[test]
    fn the_rendered_config_parses_into_typed_devices() {
        let config = parse(&rendered()).unwrap();
        assert_eq!(config.owner.as_str(), "t");
        assert_eq!(
            config.drop_zone.as_path(),
            Path::new("/var/lib/vogix/machine")
        );
        let endpoint = config.openrgb.as_ref().unwrap();
        assert_eq!(endpoint.port, 6742);
        assert_eq!(endpoint.max_protocol, MaxProtocol::V6);

        let openrgb: Vec<_> = config
            .openrgb_devices()
            .map(|(n, s, d)| {
                (
                    n.as_str(),
                    s.as_str(),
                    d.name_contains.as_str(),
                    d.mode.as_str(),
                )
            })
            .collect();
        assert_eq!(
            openrgb,
            [
                ("dram-rgb", "base01", "ENE DRAM", "Static"),
                ("keychron-k2-he", "base01", "Keychron K2 HE", "Static"),
            ]
        );

        let (name, _, kraken) = config.command_devices().next().unwrap();
        assert_eq!(name.as_str(), "kraken-ring");
        assert_eq!(
            kraken.hotplug.hidraw,
            Some(HidrawMatch {
                vendor_id: UsbId::new(0x1e71),
                product_id: UsbId::new(0x3012)
            })
        );
        assert_eq!(
            kraken
                .argv
                .render(Rgb::new(0x3b, 0x42, 0x52))
                .last()
                .unwrap(),
            "3b4252"
        );
    }

    #[test]
    fn serialization_round_trips_to_the_same_json() {
        let config = parse(&rendered()).unwrap();
        let again: serde_json::Value = serde_json::to_value(&config).unwrap();
        assert_eq!(again, rendered());
    }

    #[test]
    fn unknown_fields_are_rejected_at_every_level() {
        let mut v = rendered();
        v["extra"] = json!(1);
        rejects(v, "unknown field `extra`");

        let mut v = rendered();
        v["console"]["color"] = json!(true);
        rejects(v, "unknown field `color`");

        let mut v = rendered();
        v["openrgb"]["timeout"] = json!(5);
        rejects(v, "unknown field `timeout`");

        let mut v = rendered();
        v["devices"]["dram-rgb"]["provider"]["openrgb"]["zone"] = json!("x");
        rejects(v, "unknown field `zone`");

        let mut v = rendered();
        v["devices"]["kraken-ring"]["provider"]["command"]["hotplug"]["usb"] = json!(null);
        rejects(v, "unknown field `usb`");
    }

    #[test]
    fn a_provider_is_exactly_one_known_tag() {
        let mut v = rendered();
        v["devices"]["dram-rgb"]["provider"] = json!({ "i2c": { "bus": 1 } });
        rejects(v, "unknown variant `i2c`");

        let mut v = rendered();
        v["devices"]["dram-rgb"]["provider"] = json!({
            "openrgb": { "nameContains": "ENE DRAM", "mode": "Static" },
            "command": { "argv": ["/bin/true"] }
        });
        assert!(parse(&v).is_err(), "two tags are not one provider");
    }

    #[test]
    fn hotplug_may_be_null_or_absent() {
        let mut v = rendered();
        v["devices"]["kraken-ring"]["provider"]["command"]["hotplug"] = json!({ "hidraw": null });
        let config = parse(&v).unwrap();
        assert_eq!(
            config.command_devices().next().unwrap().2.hotplug.hidraw,
            None
        );

        let mut v = rendered();
        v["devices"]["kraken-ring"]["provider"]["command"]
            .as_object_mut()
            .unwrap()
            .remove("hotplug");
        assert!(parse(&v).is_ok());
    }

    #[test]
    fn argv_rules() {
        let with_argv = |argv: serde_json::Value| {
            let mut v = rendered();
            v["devices"]["kraken-ring"]["provider"]["command"]["argv"] = argv;
            v
        };
        rejects(
            with_argv(json!(["liquidctl", "{{color}}"])),
            "not an absolute path",
        );
        rejects(with_argv(json!([])), "argv is empty");
        rejects(with_argv(json!(["/bin/x", "a\u{0}b"])), "NUL byte");
        rejects(
            with_argv(json!(["/bin/x", "--color={{color}}"])),
            "must be a whole argument",
        );
        let ok = parse(&with_argv(json!(["/bin/x", "{{color}}", "{{color}}"]))).unwrap();
        let (_, _, device) = ok.command_devices().next().unwrap();
        assert_eq!(device.argv.program(), "/bin/x");
        assert_eq!(
            device.argv.render(Rgb::new(1, 2, 3)),
            ["/bin/x", "010203", "010203"]
        );
    }

    #[test]
    fn value_types_are_checked() {
        let mut v = rendered();
        v["devices"]["dram-rgb"]["slot"] = json!("{{base01}}");
        rejects(v, "invalid SlotName");

        let mut v = rendered();
        v["devices"]["kraken-ring"]["provider"]["command"]["hotplug"]["hidraw"]["vendorId"] =
            json!("1E71");
        rejects(v, "four lowercase hex digits");

        let mut v = rendered();
        v["openrgb"]["maxProtocol"] = json!(4);
        rejects(v, "maxProtocol 4 is not supported");

        let mut v = rendered();
        v["schema"] = json!(2);
        rejects(v, "schema 2 is not supported");

        let mut v = rendered();
        v["dropZone"] = json!("/var/lib/../tmp");
        rejects(v, "invalid drop zone");

        let mut v = rendered();
        v["dropZone"] = json!("relative/zone");
        rejects(v, "invalid drop zone");

        let mut v = rendered();
        v["devices"] = json!({ "bad name": { "slot": "base01", "provider": { "command": { "argv": ["/bin/true"] } } } });
        rejects(v, "invalid DeviceName");
    }

    #[test]
    fn cross_field_rules() {
        let mut v = rendered();
        v["openrgb"] = json!(null);
        rejects(v, "dram-rgb, keychron-k2-he use the openrgb provider");

        let mut v = rendered();
        v["openrgb"]["host"] = json!("192.168.1.2");
        rejects(v, "not a loopback address");

        let mut v = rendered();
        v["openrgb"]["port"] = json!(0);
        rejects(v, "openrgb.port is 0");

        // A config with only command devices needs no endpoint.
        let mut v = rendered();
        v["openrgb"] = json!(null);
        v["devices"].as_object_mut().unwrap().remove("dram-rgb");
        v["devices"]
            .as_object_mut()
            .unwrap()
            .remove("keychron-k2-he");
        assert!(parse(&v).is_ok());
    }

    #[test]
    fn load_follows_the_etc_symlink_and_bounds_the_file() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("store-machine.json");
        std::fs::write(&target, rendered().to_string()).unwrap();
        let link = dir.path().join("machine.json");
        symlink(&target, &link).unwrap();
        assert!(MachineConfig::load(&link).is_ok());

        let big = dir.path().join("big.json");
        std::fs::write(&big, vec![b' '; MAX_MACHINE_FILE_BYTES as usize + 1]).unwrap();
        assert!(matches!(
            MachineConfig::load(&big),
            Err(ConfigError::TooLarge { .. })
        ));

        assert!(matches!(
            MachineConfig::load(&dir.path().join("absent.json")),
            Err(ConfigError::Absent { .. })
        ));
        assert!(matches!(
            MachineConfig::load(dir.path()),
            Err(ConfigError::NotRegularFile { .. })
        ));

        let broken = dir.path().join("broken.json");
        std::fs::write(&broken, b"{\"schema\": 1").unwrap();
        let err = MachineConfig::load(&broken).unwrap_err();
        assert!(matches!(err, ConfigError::Invalid { .. }));
        assert!(err.to_string().starts_with(&broken.display().to_string()));
    }
}

//! The CLI side of the drop zone: after a theme apply, the machine owner's
//! vogix publishes the palette the machine units follow.
//!
//! Publishing is decided by ownership, not by configuration: the drop zone
//! belongs to the declared owner, so only a CLI whose effective uid owns it
//! writes there, and every other user's apply stops at an info line. The
//! file is replaced atomically and left alone when it already holds exactly
//! these bytes, so a republish of the same palette reaches no watcher.

use super::config::{ConfigError, MachineConfig};
use super::palette::{MachinePalette, PALETTE_FILE, ThemeRef};
use super::types::{Label, Rgb, SchemaV1, SlotName, UserName};
use crate::fsutil::{self, WriteOutcome};
use crate::scheme::Scheme;
use std::collections::{BTreeMap, HashMap};
use std::fs::File;
use std::io;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

/// The console palette file of a theme package (16 `#rrggbb` lines, the
/// console app's `setvtrgb` input), relative to the `current-theme` link.
pub const CONSOLE_PALETTE: &str = "console/palette";

/// A console palette file is 16 short lines.
const MAX_CONSOLE_FILE_BYTES: u64 = 4096;

/// What the palette is built from: the applied theme and its colours.
#[derive(Debug, Clone, Copy)]
pub struct Applied<'a> {
    pub scheme: Scheme,
    pub theme: &'a str,
    pub variant: &'a str,
    /// The slot → colour map the apply hooks were given; `None` when the
    /// theme's colours could not be loaded.
    pub colors: Option<&'a HashMap<String, String>>,
    /// The theme package's console palette file.
    pub console_file: &'a Path,
}

/// What [`publish`] did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Published {
    /// There is no machine config: this host has no machine surfaces.
    NotConfigured,
    /// The drop zone belongs to another user; the machine surfaces follow
    /// the declared owner.
    NotOwner { owner: UserName },
    /// The drop zone already holds exactly this palette.
    Unchanged { path: PathBuf },
    /// A new palette was renamed into the drop zone.
    Written { path: PathBuf },
}

/// Why the palette was not published.
#[derive(Debug, thiserror::Error)]
pub enum PublishError {
    #[error(transparent)]
    Config(ConfigError),
    #[error("cannot read the drop zone {path}: {source}")]
    Zone { path: PathBuf, source: io::Error },
    #[error("the theme's colours were not loaded")]
    NoColours,
    #[error("theme {what} {value:?} cannot be published: {reason}")]
    ThemeName {
        what: &'static str,
        value: String,
        reason: String,
    },
    #[error("cannot write {path}: {source}")]
    Write { path: PathBuf, source: io::Error },
}

/// Publish the applied theme's palette into the drop zone named by the
/// machine config at `config_path`, when this process's effective user owns
/// that drop zone.
pub fn publish(config_path: &Path, applied: &Applied<'_>) -> Result<Published, PublishError> {
    let config = match MachineConfig::load(config_path) {
        Ok(config) => config,
        Err(ConfigError::Absent { .. }) => return Ok(Published::NotConfigured),
        Err(e) => return Err(PublishError::Config(e)),
    };
    let zone = config.drop_zone.as_path();
    let zone_uid = std::fs::metadata(zone)
        .map_err(|source| PublishError::Zone {
            path: zone.to_path_buf(),
            source,
        })?
        .uid();
    // SAFETY: geteuid has no preconditions and cannot fail.
    let euid = unsafe { libc::geteuid() };
    if zone_uid != euid {
        return Ok(Published::NotOwner {
            owner: config.owner,
        });
    }

    let palette = build_palette(applied)?;
    let path = zone.join(PALETTE_FILE);
    match fsutil::write_atomic(&path, &palette.to_json(), 0o644) {
        Ok(WriteOutcome::Written) => Ok(Published::Written { path }),
        Ok(WriteOutcome::Unchanged) => Ok(Published::Unchanged { path }),
        Err(source) => Err(PublishError::Write { path, source }),
    }
}

/// The palette document for `applied`. A slot whose name or colour the
/// palette schema does not accept is left out with a warning; an unreadable
/// or malformed console file publishes `console: null` with a warning, which
/// leaves the VT palette alone.
fn build_palette(applied: &Applied<'_>) -> Result<MachinePalette, PublishError> {
    let colors = applied.colors.ok_or(PublishError::NoColours)?;
    let label = |what: &'static str, value: &str| {
        Label::try_from(value.to_string()).map_err(|reason| PublishError::ThemeName {
            what,
            value: value.to_string(),
            reason,
        })
    };
    let theme = ThemeRef {
        scheme: applied.scheme,
        name: label("name", applied.theme)?,
        variant: label("variant", applied.variant)?,
    };

    let mut slots = BTreeMap::new();
    for (name, value) in colors {
        match (name.parse::<SlotName>(), slot_colour(value)) {
            (Ok(slot), Some(colour)) => {
                slots.insert(slot, colour);
            }
            (Err(why), _) => log::warn!("machine palette: slot left out: {why}"),
            (Ok(_), None) => {
                log::warn!("machine palette: slot {name} left out: {value:?} is not #rrggbb")
            }
        }
    }

    let console = match read_console(applied.console_file) {
        Ok(console) => console,
        Err(why) => {
            log::warn!(
                "machine palette: no console colours from {}: {why}",
                applied.console_file.display()
            );
            None
        }
    };

    Ok(MachinePalette {
        schema: SchemaV1,
        theme,
        slots,
        console,
    })
}

/// A theme colour as the apply hooks see it: six hex digits, with or
/// without a leading '#'.
fn slot_colour(value: &str) -> Option<Rgb> {
    let hex = value.strip_prefix('#').unwrap_or(value);
    format!("#{hex}").parse().ok()
}

/// The 16 VT colours of a console palette file, or `None` when the theme
/// has no console file (the owner's console app is off).
fn read_console(path: &Path) -> Result<Option<[Rgb; 16]>, String> {
    let mut file = match File::open(path) {
        Ok(file) => file,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.to_string()),
    };
    let bytes =
        fsutil::read_capped(&mut file, MAX_CONSOLE_FILE_BYTES).map_err(|e| e.to_string())?;
    let text = std::str::from_utf8(&bytes).map_err(|e| e.to_string())?;
    parse_console(text).map(Some)
}

/// Exactly 16 `#rrggbb` lines, in ANSI colour order; blank lines are
/// ignored.
fn parse_console(text: &str) -> Result<[Rgb; 16], String> {
    let colours = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::parse::<Rgb>)
        .collect::<Result<Vec<_>, _>>()?;
    let count = colours.len();
    colours
        .try_into()
        .map_err(|_| format!("{count} colours, expected 16"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::machine::palette::MachinePalette;
    use std::os::unix::fs::PermissionsExt;

    fn console_text() -> String {
        (0..16u8)
            .map(|i| format!("#{i:02X}{:02x}{:02x}\n", i * 2, i * 3))
            .collect()
    }

    fn colors() -> HashMap<String, String> {
        [
            ("base01", "#3b4252"),
            ("base0D", "#81A1C1"),
            ("foreground_text", "d8dee9"),
            ("bad name", "#000000"),
            ("base02", "#12345"),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
    }

    /// A drop zone owned by this user and a machine config naming it.
    struct Host {
        dir: tempfile::TempDir,
    }

    impl Host {
        fn new() -> Self {
            let dir = tempfile::tempdir().unwrap();
            std::fs::create_dir(dir.path().join("zone")).unwrap();
            let host = Self { dir };
            host.config_naming(&host.zone());
            host
        }

        fn zone(&self) -> PathBuf {
            self.dir.path().join("zone")
        }

        fn config(&self) -> PathBuf {
            self.dir.path().join("machine.json")
        }

        fn config_naming(&self, zone: &Path) {
            let json = serde_json::json!({
                "schema": 1, "owner": "t", "dropZone": zone,
                "console": { "enable": true }, "openrgb": null, "devices": {}
            });
            std::fs::write(self.config(), json.to_string()).unwrap();
        }

        fn console_file(&self, text: &str) -> PathBuf {
            let path = self.dir.path().join("palette");
            std::fs::write(&path, text).unwrap();
            path
        }
    }

    fn applied<'a>(colors: &'a HashMap<String, String>, console: &'a Path) -> Applied<'a> {
        Applied {
            scheme: Scheme::Vogix16,
            theme: "nordic",
            variant: "dark",
            colors: Some(colors),
            console_file: console,
        }
    }

    #[test]
    fn the_owner_publishes_a_palette_the_owners_loader_accepts() {
        let host = Host::new();
        let colors = colors();
        let console = host.console_file(&console_text());
        let path = match publish(&host.config(), &applied(&colors, &console)).unwrap() {
            Published::Written { path } => path,
            other => panic!("expected a write, got {other:?}"),
        };
        assert_eq!(path, host.zone().join(PALETTE_FILE));
        assert_eq!(
            std::fs::metadata(&path).unwrap().permissions().mode() & 0o7777,
            0o644
        );

        let palette = MachinePalette::load_from_zone(&host.zone()).unwrap();
        assert_eq!(palette.theme.name.as_str(), "nordic");
        assert_eq!(palette.theme.variant.as_str(), "dark");
        assert_eq!(palette.theme.scheme, Scheme::Vogix16);
        let slots: Vec<(&str, String)> = palette
            .slots
            .iter()
            .map(|(k, v)| (k.as_str(), v.to_string()))
            .collect();
        assert_eq!(
            slots,
            [
                ("base01", "#3b4252".to_string()),
                ("base0D", "#81a1c1".to_string()),
                ("foreground_text", "#d8dee9".to_string()),
            ]
        );
        let console = palette.console.unwrap();
        assert_eq!(console[0], Rgb::new(0, 0, 0));
        assert_eq!(console[15], Rgb::new(15, 30, 45));
    }

    #[test]
    fn republishing_the_same_palette_is_unchanged_and_keeps_the_inode() {
        let host = Host::new();
        let colors = colors();
        let console = host.console_file(&console_text());
        publish(&host.config(), &applied(&colors, &console)).unwrap();
        let inode = |p: &Path| std::fs::metadata(p).unwrap().ino();
        let before = inode(&host.zone().join(PALETTE_FILE));
        assert!(matches!(
            publish(&host.config(), &applied(&colors, &console)).unwrap(),
            Published::Unchanged { .. }
        ));
        assert_eq!(inode(&host.zone().join(PALETTE_FILE)), before);
    }

    #[test]
    fn no_machine_config_means_not_configured() {
        let host = Host::new();
        let colors = colors();
        let console = host.console_file(&console_text());
        let absent = host.dir.path().join("absent.json");
        assert_eq!(
            publish(&absent, &applied(&colors, &console)).unwrap(),
            Published::NotConfigured
        );
        assert!(!host.zone().join(PALETTE_FILE).exists());
    }

    #[test]
    fn a_drop_zone_of_another_user_is_not_written() {
        // /dev/shm is root-owned (or, inside a build sandbox's user
        // namespace, owned by an unmapped uid): a drop zone this user does
        // not own.
        let shm = Path::new("/dev/shm");
        // SAFETY: geteuid has no preconditions.
        let euid = unsafe { libc::geteuid() };
        assert_ne!(
            std::fs::metadata(shm).unwrap().uid(),
            euid,
            "the test needs a directory owned by another uid"
        );
        let host = Host::new();
        host.config_naming(shm);
        let colors = colors();
        let console = host.console_file(&console_text());
        assert_eq!(
            publish(&host.config(), &applied(&colors, &console)).unwrap(),
            Published::NotOwner {
                owner: "t".parse().unwrap()
            }
        );
    }

    #[test]
    fn an_invalid_machine_config_is_an_error() {
        let host = Host::new();
        std::fs::write(host.config(), br#"{"schema": 2}"#).unwrap();
        let colors = colors();
        let console = host.console_file(&console_text());
        let err = publish(&host.config(), &applied(&colors, &console)).unwrap_err();
        assert!(matches!(err, PublishError::Config(_)), "{err}");
    }

    #[test]
    fn a_missing_drop_zone_is_an_error_naming_it() {
        let host = Host::new();
        std::fs::remove_dir(host.zone()).unwrap();
        let colors = colors();
        let console = host.console_file(&console_text());
        let err = publish(&host.config(), &applied(&colors, &console))
            .unwrap_err()
            .to_string();
        assert!(err.contains(&host.zone().display().to_string()), "{err}");
    }

    #[test]
    fn without_colours_nothing_is_published() {
        let host = Host::new();
        let console = host.console_file(&console_text());
        let colors = colors();
        let applied = Applied {
            colors: None,
            ..applied(&colors, &console)
        };
        assert!(matches!(
            publish(&host.config(), &applied),
            Err(PublishError::NoColours)
        ));
        assert!(!host.zone().join(PALETTE_FILE).exists());
    }

    #[test]
    fn a_theme_without_a_console_file_publishes_no_console() {
        let host = Host::new();
        let colors = colors();
        let absent = host.dir.path().join("no-console");
        publish(&host.config(), &applied(&colors, &absent)).unwrap();
        let palette = MachinePalette::load_from_zone(&host.zone()).unwrap();
        assert_eq!(palette.console, None);
        assert!(!palette.slots.is_empty());
    }

    #[test]
    fn a_malformed_console_file_publishes_no_console() {
        let host = Host::new();
        let colors = colors();
        let short: String = console_text()
            .lines()
            .take(15)
            .collect::<Vec<_>>()
            .join("\n");
        let console = host.console_file(&short);
        publish(&host.config(), &applied(&colors, &console)).unwrap();
        let palette = MachinePalette::load_from_zone(&host.zone()).unwrap();
        assert_eq!(palette.console, None);
    }

    #[test]
    fn the_console_file_is_sixteen_colours_in_order() {
        let parsed = parse_console(&console_text()).unwrap();
        assert_eq!(parsed[1], Rgb::new(1, 2, 3));
        // The file as the console app renders it: no trailing newline.
        assert!(parse_console(console_text().trim_end()).is_ok());
        let err = parse_console(&format!("{}#ffffff\n", console_text())).unwrap_err();
        assert_eq!(err, "17 colours, expected 16");
        assert!(parse_console("#12345g\n").is_err());
    }

    #[test]
    fn slot_colours_take_an_optional_hash() {
        assert_eq!(slot_colour("#3B4252"), Some(Rgb::new(0x3b, 0x42, 0x52)));
        assert_eq!(slot_colour("3b4252"), Some(Rgb::new(0x3b, 0x42, 0x52)));
        for bad in ["", "#", "##3b4252", "3b425", "3b42520", "rgb(1,2,3)"] {
            assert_eq!(slot_colour(bad), None, "{bad:?}");
        }
    }
}

//! `palette.json` in the drop zone: the colours the machine owner published.
//!
//! The file is written by a user and read by system units, so the reader
//! trusts nothing about it: the file is opened relative to the drop-zone
//! directory with `O_NOFOLLOW` (a symlink is refused) and `O_NONBLOCK` (a
//! FIFO cannot stall the reader), and it is accepted only when it is a
//! regular file owned by the directory's owner, at most
//! [`MAX_MACHINE_FILE_BYTES`], and a schema-1 document with no unknown
//! fields.

use super::config::MAX_MACHINE_FILE_BYTES;
use super::types::{Label, Rgb, SchemaV1, SlotName};
use crate::fsutil;
use crate::scheme::Scheme;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::ffi::CString;
use std::fs::{File, Metadata, OpenOptions};
use std::io;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::{Path, PathBuf};

/// The file name the owner publishes inside the drop zone.
pub const PALETTE_FILE: &str = "palette.json";

/// The published colours: the theme they come from, every palette slot, and
/// the 16 VT colours (null when the owner's console app is off).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MachinePalette {
    pub schema: SchemaV1,
    pub theme: ThemeRef,
    pub slots: BTreeMap<SlotName, Rgb>,
    pub console: Option<[Rgb; 16]>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThemeRef {
    pub scheme: Scheme,
    pub name: Label,
    pub variant: Label,
}

/// Why a published palette was not accepted.
#[derive(Debug, thiserror::Error)]
pub enum PaletteError {
    #[error("{path} does not exist")]
    Absent { path: PathBuf },
    #[error("{path} is a symlink; the palette must be a regular file")]
    Symlink { path: PathBuf },
    #[error("{path} is not a regular file")]
    NotRegularFile { path: PathBuf },
    #[error("{path} is owned by uid {file_uid}, not by the drop zone's owner (uid {zone_uid})")]
    ForeignOwner {
        path: PathBuf,
        file_uid: u32,
        zone_uid: u32,
    },
    #[error("{path} is {size} bytes; the limit is {max}")]
    TooLarge { path: PathBuf, size: u64, max: u64 },
    #[error("cannot read {path}: {source}")]
    Io { path: PathBuf, source: io::Error },
    #[error("{path}: {source}")]
    Invalid {
        path: PathBuf,
        source: serde_json::Error,
    },
}

impl MachinePalette {
    /// The colour of `slot`, if the published theme has that slot.
    pub fn slot(&self, slot: &SlotName) -> Option<Rgb> {
        self.slots.get(slot).copied()
    }

    /// Load `<zone>/palette.json`.
    pub fn load_from_zone(zone: &Path) -> Result<Self, PaletteError> {
        Self::load_file(&zone.join(PALETTE_FILE))
    }

    /// Load a palette file, accepting it only on the drop-zone terms: its
    /// directory is opened first (following links in the directory's own
    /// path), and the file is opened relative to that directory without
    /// following a link, so the ownership check compares the file with the
    /// very directory it was found in.
    pub fn load_file(path: &Path) -> Result<Self, PaletteError> {
        let io_err = |at: &Path, source: io::Error| PaletteError::Io {
            path: at.to_path_buf(),
            source,
        };
        let name = path
            .file_name()
            .ok_or_else(|| PaletteError::NotRegularFile {
                path: path.to_path_buf(),
            })?;
        let dir_path = match path.parent() {
            Some(parent) if !parent.as_os_str().is_empty() => parent,
            _ => Path::new("."),
        };
        let dir = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY)
            .open(dir_path)
            .map_err(|e| io_err(dir_path, e))?;
        let zone_uid = dir.metadata().map_err(|e| io_err(dir_path, e))?.uid();

        let mut file = open_beneath(&dir, name.as_bytes()).map_err(|e| match e.raw_os_error() {
            Some(libc::ENOENT) => PaletteError::Absent {
                path: path.to_path_buf(),
            },
            Some(libc::ELOOP) => PaletteError::Symlink {
                path: path.to_path_buf(),
            },
            _ => io_err(path, e),
        })?;
        let meta = file.metadata().map_err(|e| io_err(path, e))?;
        FileFacts::of(&meta).accept(zone_uid, path)?;

        let bytes = fsutil::read_capped(&mut file, MAX_MACHINE_FILE_BYTES).map_err(|e| {
            if e.kind() == io::ErrorKind::InvalidData {
                // The file grew past the cap after the fstat.
                PaletteError::TooLarge {
                    path: path.to_path_buf(),
                    size: MAX_MACHINE_FILE_BYTES + 1,
                    max: MAX_MACHINE_FILE_BYTES,
                }
            } else {
                io_err(path, e)
            }
        })?;
        serde_json::from_slice(&bytes).map_err(|source| PaletteError::Invalid {
            path: path.to_path_buf(),
            source,
        })
    }

    /// The canonical file bytes: pretty JSON with a trailing newline. Slots
    /// are a `BTreeMap`, so equal palettes give equal bytes and a republish
    /// of the same palette is skipped by the atomic writer.
    pub fn to_json(&self) -> Vec<u8> {
        let mut bytes =
            serde_json::to_vec_pretty(self).expect("a MachinePalette always serializes");
        bytes.push(b'\n');
        bytes
    }
}

/// `openat(dir, name, O_RDONLY | O_NOFOLLOW | O_NONBLOCK | O_NOCTTY | O_CLOEXEC)`.
fn open_beneath(dir: &File, name: &[u8]) -> io::Result<File> {
    let name = CString::new(name).map_err(|_| {
        io::Error::new(io::ErrorKind::InvalidInput, "file name contains a NUL byte")
    })?;
    // SAFETY: `dir` is an open directory fd and `name` a NUL-terminated
    // string, both alive for the call; openat has no other preconditions.
    let fd = unsafe {
        libc::openat(
            dir.as_raw_fd(),
            name.as_ptr(),
            libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_NOCTTY | libc::O_CLOEXEC,
        )
    };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: openat returned a fresh fd that nothing else owns.
    Ok(File::from(unsafe { OwnedFd::from_raw_fd(fd) }))
}

/// What the acceptance rule looks at, taken from the opened file's fstat.
#[derive(Debug, Clone, Copy)]
struct FileFacts {
    regular: bool,
    uid: u32,
    size: u64,
}

impl FileFacts {
    fn of(meta: &Metadata) -> Self {
        Self {
            regular: meta.is_file(),
            uid: meta.uid(),
            size: meta.len(),
        }
    }

    fn accept(self, zone_uid: u32, path: &Path) -> Result<(), PaletteError> {
        let path = path.to_path_buf();
        if !self.regular {
            return Err(PaletteError::NotRegularFile { path });
        }
        if self.uid != zone_uid {
            return Err(PaletteError::ForeignOwner {
                path,
                file_uid: self.uid,
                zone_uid,
            });
        }
        if self.size > MAX_MACHINE_FILE_BYTES {
            return Err(PaletteError::TooLarge {
                path,
                size: self.size,
                max: MAX_MACHINE_FILE_BYTES,
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::os::unix::fs::symlink;

    fn console() -> Vec<String> {
        (0..16u8)
            .map(|i| format!("#{i:02x}{i:02x}{i:02x}"))
            .collect()
    }

    fn published() -> serde_json::Value {
        json!({
            "schema": 1,
            "theme": { "scheme": "vogix16", "name": "nordic", "variant": "dark" },
            "slots": { "base01": "#3b4252", "base0D": "#81A1C1", "foreground_text": "#d8dee9" },
            "console": console()
        })
    }

    fn write(dir: &Path, value: &serde_json::Value) -> PathBuf {
        let path = dir.join(PALETTE_FILE);
        fsutil::write_atomic(&path, value.to_string().as_bytes(), 0o644).unwrap();
        path
    }

    fn load(value: serde_json::Value) -> Result<MachinePalette, PaletteError> {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), &value);
        MachinePalette::load_from_zone(dir.path())
    }

    #[test]
    fn a_published_palette_loads_typed() {
        let palette = load(published()).unwrap();
        assert_eq!(palette.theme.scheme, Scheme::Vogix16);
        assert_eq!(palette.theme.name.as_str(), "nordic");
        assert_eq!(
            palette.slot(&"base0D".parse().unwrap()),
            Some(Rgb::new(0x81, 0xa1, 0xc1))
        );
        assert_eq!(palette.slot(&"base0F".parse().unwrap()), None);
        assert_eq!(palette.console.unwrap()[15], Rgb::new(15, 15, 15));
    }

    #[test]
    fn canonical_bytes_round_trip_and_are_stable() {
        let palette = load(published()).unwrap();
        let bytes = palette.to_json();
        assert_eq!(bytes.last(), Some(&b'\n'));
        let again: MachinePalette = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(again, palette);
        assert_eq!(again.to_json(), bytes);
        assert!(
            String::from_utf8(bytes).unwrap().contains("\"#81a1c1\""),
            "colours are written in lowercase"
        );
    }

    #[test]
    fn the_console_is_null_or_exactly_sixteen_colours() {
        let mut v = published();
        v["console"] = json!(null);
        assert_eq!(load(v).unwrap().console, None);

        for n in [15, 17, 0] {
            let mut v = published();
            v["console"] = json!(console().into_iter().cycle().take(n).collect::<Vec<_>>());
            assert!(
                matches!(load(v), Err(PaletteError::Invalid { .. })),
                "{n} console colours"
            );
        }
    }

    #[test]
    fn unknown_fields_and_bad_values_are_rejected() {
        let invalid = |v: serde_json::Value, needle: &str| {
            let err = load(v).expect_err("must be rejected");
            assert!(matches!(err, PaletteError::Invalid { .. }), "{err}");
            assert!(err.to_string().contains(needle), "{err} lacks {needle:?}");
        };
        let mut v = published();
        v["publisher"] = json!("logger");
        invalid(v, "unknown field `publisher`");

        let mut v = published();
        v["theme"]["polarity"] = json!("dark");
        invalid(v, "unknown field `polarity`");

        let mut v = published();
        v["slots"]["base01"] = json!("3b4252");
        invalid(v, "expected #rrggbb");

        let mut v = published();
        v["slots"]["base 01"] = json!("#3b4252");
        invalid(v, "invalid SlotName");

        let mut v = published();
        v["theme"]["scheme"] = json!("base32");
        invalid(v, "unknown variant `base32`");

        let mut v = published();
        v["schema"] = json!(0);
        invalid(v, "schema 0 is not supported");
    }

    #[test]
    fn a_symlink_is_refused_even_to_a_valid_palette() {
        let dir = tempfile::tempdir().unwrap();
        let elsewhere = tempfile::tempdir().unwrap();
        let target = write(elsewhere.path(), &published());
        symlink(&target, dir.path().join(PALETTE_FILE)).unwrap();
        assert!(matches!(
            MachinePalette::load_from_zone(dir.path()),
            Err(PaletteError::Symlink { .. })
        ));
    }

    #[test]
    fn a_fifo_is_refused_without_blocking() {
        let dir = tempfile::tempdir().unwrap();
        let fifo = CString::new(dir.path().join(PALETTE_FILE).as_os_str().as_bytes()).unwrap();
        // SAFETY: a valid NUL-terminated path.
        assert_eq!(unsafe { libc::mkfifo(fifo.as_ptr(), 0o644) }, 0);
        assert!(matches!(
            MachinePalette::load_from_zone(dir.path()),
            Err(PaletteError::NotRegularFile { .. })
        ));
    }

    #[test]
    fn an_oversized_file_is_refused_before_it_is_read() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join(PALETTE_FILE),
            vec![b' '; MAX_MACHINE_FILE_BYTES as usize + 1],
        )
        .unwrap();
        assert!(matches!(
            MachinePalette::load_from_zone(dir.path()),
            Err(PaletteError::TooLarge { size, .. }) if size == MAX_MACHINE_FILE_BYTES + 1
        ));
    }

    #[test]
    fn an_absent_palette_is_its_own_error() {
        let dir = tempfile::tempdir().unwrap();
        assert!(matches!(
            MachinePalette::load_from_zone(dir.path()),
            Err(PaletteError::Absent { .. })
        ));
    }

    #[test]
    fn a_file_not_owned_by_the_directory_owner_is_refused() {
        // /dev/shm is root-owned (or, inside a build sandbox's user
        // namespace, owned by an unmapped uid) and world-writable, so a file
        // this test creates there has a different owner than its directory.
        let shm = Path::new("/dev/shm");
        let zone_uid = std::fs::metadata(shm).unwrap().uid();
        // SAFETY: geteuid has no preconditions.
        let euid = unsafe { libc::geteuid() };
        assert_ne!(
            zone_uid, euid,
            "the test needs a directory owned by another uid"
        );

        let file = tempfile::Builder::new()
            .prefix("vogix-palette-test-")
            .suffix(".json")
            .tempfile_in(shm)
            .unwrap();
        std::fs::write(file.path(), published().to_string()).unwrap();
        match MachinePalette::load_file(file.path()) {
            Err(PaletteError::ForeignOwner {
                file_uid,
                zone_uid: z,
                ..
            }) => {
                assert_eq!(file_uid, euid);
                assert_eq!(z, zone_uid);
            }
            other => panic!("expected ForeignOwner, got {other:?}"),
        }
    }

    #[test]
    fn the_acceptance_rule_checks_kind_then_owner_then_size() {
        let p = Path::new("/zone/palette.json");
        let ok = FileFacts {
            regular: true,
            uid: 1000,
            size: 10,
        };
        assert!(ok.accept(1000, p).is_ok());
        assert!(matches!(
            FileFacts {
                regular: false,
                ..ok
            }
            .accept(1000, p),
            Err(PaletteError::NotRegularFile { .. })
        ));
        assert!(matches!(
            ok.accept(0, p),
            Err(PaletteError::ForeignOwner {
                file_uid: 1000,
                zone_uid: 0,
                ..
            })
        ));
        assert!(matches!(
            FileFacts {
                size: MAX_MACHINE_FILE_BYTES + 1,
                ..ok
            }
            .accept(1000, p),
            Err(PaletteError::TooLarge { .. })
        ));
        assert!(
            FileFacts {
                size: MAX_MACHINE_FILE_BYTES,
                ..ok
            }
            .accept(1000, p)
            .is_ok()
        );
    }
}

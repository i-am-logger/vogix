//! The machine owners' status file, `status.json` in each owner's runtime
//! directory — what `vogix machine status` reads.
//!
//! Each owner rewrites it (atomically, identical bytes skipped) whenever its
//! state changes, and sends the same state as its `STATUS=` line.

use super::config::{MAX_MACHINE_FILE_BYTES, MachineConfig};
use super::palette::ThemeRef;
use super::types::{DeviceName, SchemaV1};
use crate::fsutil::{self, WriteOutcome};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::fmt;
use std::fs::File;
use std::io;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};

/// The file name inside an owner's runtime directory.
pub const STATUS_FILE: &str = "status.json";

/// Which machine owner unit a status file belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OwnerUnit {
    /// `vogix-openrgb.service`: the OpenRGB SDK client.
    Openrgb,
    /// `vogix-machine.service`: the VT palette and the command devices.
    Local,
}

impl OwnerUnit {
    pub const ALL: [Self; 2] = [Self::Openrgb, Self::Local];

    pub const fn unit_name(self) -> &'static str {
        match self {
            Self::Openrgb => "vogix-openrgb.service",
            Self::Local => "vogix-machine.service",
        }
    }

    /// The unit's `RuntimeDirectory=`.
    pub fn runtime_dir(self) -> &'static Path {
        Path::new(match self {
            Self::Openrgb => "/run/vogix/openrgb",
            Self::Local => "/run/vogix/machine",
        })
    }

    pub fn status_path(self) -> PathBuf {
        self.runtime_dir().join(STATUS_FILE)
    }

    /// Whether a machine config gives this owner anything to own, which is
    /// when the NixOS module creates its unit: the OpenRGB owner for any
    /// openrgb device, the local owner for the VT palette or any command
    /// device.
    pub fn is_declared_by(self, config: &MachineConfig) -> bool {
        match self {
            Self::Openrgb => config.openrgb_devices().next().is_some(),
            Self::Local => config.console.enable || config.command_devices().next().is_some(),
        }
    }
}

/// Where an owner is in its life.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Phase {
    Starting,
    /// No palette has been published into the drop zone yet.
    WaitingForPalette,
    /// Connected and learning the device list (OpenRGB handshake or resync).
    Syncing,
    Ready,
    /// A deterministic fault the owner detected; it stays so until a reload,
    /// a stop or a server restart.
    Faulted,
    Stopping,
}

impl fmt::Display for Phase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Starting => "starting",
            Self::WaitingForPalette => "waiting for owner palette",
            Self::Syncing => "syncing",
            Self::Ready => "ready",
            Self::Faulted => "faulted",
            Self::Stopping => "stopping",
        })
    }
}

/// Where one surface's colour stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SurfaceState {
    /// Nothing to apply yet (no palette, or the device's slot is not in it).
    Waiting,
    /// A write or a command run is in flight.
    Pending,
    /// Written with no acknowledgement to confirm it (OpenRGB protocol 5).
    Sent,
    /// Read back, acknowledged or exited 0.
    Confirmed,
    /// No controller matches the device.
    Absent,
    Error,
}

impl fmt::Display for SurfaceState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Waiting => "waiting",
            Self::Pending => "pending",
            Self::Sent => "sent",
            Self::Confirmed => "confirmed",
            Self::Absent => "absent",
            Self::Error => "error",
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SurfaceStatus {
    pub state: SurfaceState,
    /// For an OpenRGB device, how many controllers matched it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub controllers: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

impl SurfaceStatus {
    pub fn new(state: SurfaceState) -> Self {
        Self {
            state,
            controllers: None,
            detail: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct StatusFile {
    pub schema: SchemaV1,
    pub owner: OwnerUnit,
    pub phase: Phase,
    /// Why the owner is faulted, or what it is waiting for.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    /// The theme of the palette last applied.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub theme: Option<ThemeRef>,
    /// The negotiated OpenRGB SDK protocol version.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub protocol: Option<u32>,
    /// The name the OpenRGB server reports.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub server_name: Option<String>,
    /// How many controllers the OpenRGB server lists.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub controller_count: Option<u32>,
    /// The VT palette, for the local owner with the console enabled.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub console: Option<SurfaceStatus>,
    pub devices: BTreeMap<DeviceName, SurfaceStatus>,
}

/// Why a status file could not be read.
#[derive(Debug, thiserror::Error)]
pub enum StatusError {
    #[error("{path} does not exist")]
    Absent { path: PathBuf },
    #[error("cannot read {path}: {source}")]
    Io { path: PathBuf, source: io::Error },
    #[error("{path}: {source}")]
    Invalid {
        path: PathBuf,
        source: serde_json::Error,
    },
}

impl StatusFile {
    pub fn new(owner: OwnerUnit, phase: Phase) -> Self {
        Self {
            schema: SchemaV1,
            owner,
            phase,
            detail: None,
            theme: None,
            protocol: None,
            server_name: None,
            controller_count: None,
            console: None,
            devices: BTreeMap::new(),
        }
    }

    /// Everything that makes `vogix machine status` exit 1: a fault, or a
    /// surface in error. Absent devices and in-flight writes are not
    /// problems.
    pub fn problems(&self) -> Vec<String> {
        let mut problems = Vec::new();
        if self.phase == Phase::Faulted {
            problems.push(with_detail("faulted".into(), self.detail.as_deref()));
        }
        if let Some(console) = &self.console
            && console.state == SurfaceState::Error
        {
            problems.push(with_detail(
                "console: error".into(),
                console.detail.as_deref(),
            ));
        }
        for (name, device) in &self.devices {
            if device.state == SurfaceState::Error {
                problems.push(with_detail(
                    format!("{name}: error"),
                    device.detail.as_deref(),
                ));
            }
        }
        problems
    }

    /// The one-line summary sent as `STATUS=`, e.g. `protocol 6; 3
    /// controllers; dram-rgb confirmed x2; keychron-k2-he absent`.
    pub fn summary(&self) -> String {
        let mut parts = Vec::new();
        if self.phase != Phase::Ready {
            parts.push(with_detail(self.phase.to_string(), self.detail.as_deref()));
        }
        if let Some(protocol) = self.protocol {
            parts.push(format!("protocol {protocol}"));
        }
        if let Some(count) = self.controller_count {
            parts.push(format!(
                "{count} controller{}",
                if count == 1 { "" } else { "s" }
            ));
        }
        if let Some(console) = &self.console {
            parts.push(surface_summary("console", console));
        }
        for (name, device) in &self.devices {
            parts.push(surface_summary(name.as_str(), device));
        }
        parts.join("; ")
    }

    /// The file bytes: pretty JSON with a trailing newline.
    pub fn to_json(&self) -> Vec<u8> {
        let mut bytes = serde_json::to_vec_pretty(self).expect("a StatusFile always serializes");
        bytes.push(b'\n');
        bytes
    }

    /// Write `<dir>/status.json`, world-readable, atomically.
    pub fn write(&self, dir: &Path) -> io::Result<WriteOutcome> {
        fsutil::write_atomic(&dir.join(STATUS_FILE), &self.to_json(), 0o644)
    }

    /// Read a status file.
    pub fn read(path: &Path) -> Result<Self, StatusError> {
        let io_err = |source: io::Error| match source.kind() {
            io::ErrorKind::NotFound => StatusError::Absent {
                path: path.to_path_buf(),
            },
            _ => StatusError::Io {
                path: path.to_path_buf(),
                source,
            },
        };
        let mut file = File::open(path).map_err(io_err)?;
        let bytes = fsutil::read_capped(&mut file, MAX_MACHINE_FILE_BYTES).map_err(io_err)?;
        serde_json::from_slice(&bytes).map_err(|source| StatusError::Invalid {
            path: path.to_path_buf(),
            source,
        })
    }
}

fn with_detail(text: String, detail: Option<&str>) -> String {
    match detail {
        Some(detail) => format!("{text}: {detail}"),
        None => text,
    }
}

fn surface_summary(name: &str, surface: &SurfaceStatus) -> String {
    let mut text = format!("{name} {}", surface.state);
    if let Some(n) = surface.controllers.filter(|&n| n > 1) {
        text.push_str(&format!(" x{n}"));
    }
    if surface.state == SurfaceState::Error {
        text = with_detail(text, surface.detail.as_deref());
    }
    text
}

/// The directory systemd created for the unit's `RuntimeDirectory=`
/// (`$RUNTIME_DIRECTORY`; the first entry when the unit declares several).
pub fn runtime_directory() -> io::Result<PathBuf> {
    runtime_directory_from(std::env::var_os("RUNTIME_DIRECTORY").as_deref())
}

fn runtime_directory_from(value: Option<&OsStr>) -> io::Result<PathBuf> {
    let missing = || {
        io::Error::new(
            io::ErrorKind::NotFound,
            "RUNTIME_DIRECTORY is not set: run the owner as its systemd unit",
        )
    };
    let value = value.ok_or_else(missing)?;
    let first = value.as_bytes().split(|&b| b == b':').next().unwrap_or(&[]);
    if first.is_empty() {
        return Err(missing());
    }
    Ok(PathBuf::from(OsStr::from_bytes(first)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scheme::Scheme;

    fn name(s: &str) -> DeviceName {
        s.parse().unwrap()
    }

    fn openrgb_ready() -> StatusFile {
        let mut status = StatusFile::new(OwnerUnit::Openrgb, Phase::Ready);
        status.protocol = Some(6);
        status.server_name = Some("OpenRGB".into());
        status.controller_count = Some(3);
        status.theme = Some(ThemeRef {
            scheme: Scheme::Vogix16,
            name: "nordic".parse().unwrap(),
            variant: "dark".parse().unwrap(),
        });
        status.devices.insert(
            name("dram-rgb"),
            SurfaceStatus {
                state: SurfaceState::Confirmed,
                controllers: Some(2),
                detail: None,
            },
        );
        status.devices.insert(
            name("keychron-k2-he"),
            SurfaceStatus {
                state: SurfaceState::Absent,
                controllers: Some(0),
                detail: Some("no controller matches \"Keychron K2 HE\"".into()),
            },
        );
        status
    }

    #[test]
    fn the_summary_is_the_status_line() {
        assert_eq!(
            openrgb_ready().summary(),
            "protocol 6; 3 controllers; dram-rgb confirmed x2; keychron-k2-he absent"
        );

        let mut waiting = StatusFile::new(OwnerUnit::Local, Phase::WaitingForPalette);
        waiting.console = Some(SurfaceStatus::new(SurfaceState::Waiting));
        waiting.devices.insert(
            name("kraken-ring"),
            SurfaceStatus::new(SurfaceState::Waiting),
        );
        assert_eq!(
            waiting.summary(),
            "waiting for owner palette; console waiting; kraken-ring waiting"
        );

        let mut faulted = StatusFile::new(OwnerUnit::Openrgb, Phase::Faulted);
        faulted.detail = Some("bad magic \"XRGB\"".into());
        assert_eq!(faulted.summary(), "faulted: bad magic \"XRGB\"");
    }

    #[test]
    fn faults_and_errors_are_problems_absence_is_not() {
        assert!(openrgb_ready().problems().is_empty());

        let mut status = openrgb_ready();
        status.devices.get_mut(&name("dram-rgb")).unwrap().state = SurfaceState::Error;
        status.devices.get_mut(&name("dram-rgb")).unwrap().detail =
            Some("server holds Direct, expected Static".into());
        assert_eq!(
            status.problems(),
            ["dram-rgb: error: server holds Direct, expected Static"]
        );
        assert!(
            status
                .summary()
                .contains("dram-rgb error x2: server holds Direct")
        );

        let mut local = StatusFile::new(OwnerUnit::Local, Phase::Ready);
        local.console = Some(SurfaceStatus {
            state: SurfaceState::Error,
            controllers: None,
            detail: Some("PIO_CMAP: Operation not permitted".into()),
        });
        assert_eq!(
            local.problems(),
            ["console: error: PIO_CMAP: Operation not permitted"]
        );

        let mut faulted = StatusFile::new(OwnerUnit::Openrgb, Phase::Faulted);
        faulted.detail = Some("server speaks protocol 4".into());
        assert_eq!(faulted.problems(), ["faulted: server speaks protocol 4"]);
    }

    #[test]
    fn written_status_reads_back_equal_and_rewrites_are_skipped() {
        let dir = tempfile::tempdir().unwrap();
        let status = openrgb_ready();
        assert_eq!(status.write(dir.path()).unwrap(), WriteOutcome::Written);
        assert_eq!(status.write(dir.path()).unwrap(), WriteOutcome::Unchanged);
        let path = dir.path().join(STATUS_FILE);
        assert_eq!(StatusFile::read(&path).unwrap(), status);

        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o644);

        let json: serde_json::Value = serde_json::from_slice(&status.to_json()).unwrap();
        assert_eq!(json["owner"], "openrgb");
        assert_eq!(json["phase"], "ready");
        assert_eq!(json["serverName"], "OpenRGB");
        assert_eq!(json["devices"]["dram-rgb"]["state"], "confirmed");
        assert_eq!(json["devices"]["dram-rgb"]["controllers"], 2);
        assert!(json.get("console").is_none());
    }

    #[test]
    fn reading_rejects_unknown_fields_and_reports_absence() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(STATUS_FILE);
        assert!(matches!(
            StatusFile::read(&path),
            Err(StatusError::Absent { .. })
        ));

        let mut json: serde_json::Value =
            serde_json::from_slice(&openrgb_ready().to_json()).unwrap();
        json["devices"]["dram-rgb"]["colour"] = serde_json::json!("#000000");
        std::fs::write(&path, json.to_string()).unwrap();
        let err = StatusFile::read(&path).unwrap_err();
        assert!(matches!(err, StatusError::Invalid { .. }));
        assert!(err.to_string().contains("unknown field `colour`"), "{err}");

        std::fs::write(
            &path,
            br#"{"schema":1,"owner":"local","phase":"napping","devices":{}}"#,
        )
        .unwrap();
        assert!(
            StatusFile::read(&path)
                .unwrap_err()
                .to_string()
                .contains("unknown variant `napping`")
        );
    }

    #[test]
    fn owner_units_know_their_files() {
        assert_eq!(
            OwnerUnit::Openrgb.status_path(),
            Path::new("/run/vogix/openrgb/status.json")
        );
        assert_eq!(OwnerUnit::Local.unit_name(), "vogix-machine.service");
        assert_eq!(
            OwnerUnit::Local.status_path(),
            Path::new("/run/vogix/machine/status.json")
        );
    }

    #[test]
    fn the_runtime_directory_is_the_first_entry() {
        assert_eq!(
            runtime_directory_from(Some(OsStr::new("/run/vogix/machine"))).unwrap(),
            Path::new("/run/vogix/machine")
        );
        assert_eq!(
            runtime_directory_from(Some(OsStr::new("/run/a:/run/b"))).unwrap(),
            Path::new("/run/a")
        );
        assert!(runtime_directory_from(None).is_err());
        assert!(runtime_directory_from(Some(OsStr::new(""))).is_err());
    }
}

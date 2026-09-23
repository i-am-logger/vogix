//! `vogix machine` — the command-line side of the machine surfaces.
//!
//! - `serve local` and `serve openrgb` run a machine owner as its unit
//!   starts it; the process exits with the owner's status
//!   ([`OwnerExit::code`]).
//! - `status` reads the machine config, the published palette and each
//!   declared owner unit's status file.
//! - `validate` runs the loaders the machine owners use on a given file, so a
//!   rendered `/etc/vogix/machine.json` or a published `palette.json` is
//!   accepted or rejected here exactly as the owners would.
//! - `inspect` lists what the OpenRGB server serves through a read-only
//!   client, and can capture the raw controller payloads.

use crate::cli::{MachineCommands, ServeCommands};
use crate::errors::{Result, VogixError};
use crate::machine::config::{ConfigError, MACHINE_CONFIG_PATH, MachineConfig, Provider};
use crate::machine::local;
use crate::machine::openrgb::inspect;
use crate::machine::openrgb::model::{ClientName, ProtocolVersion};
use crate::machine::openrgb::owner::{self, OwnerExit};
use crate::machine::openrgb::session::MirrorConfig;
use crate::machine::palette::{MachinePalette, PALETTE_FILE, PaletteError};
use crate::machine::status::{OwnerUnit, StatusError, StatusFile, SurfaceStatus};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

/// The name the inspect client announces, beside the owner's.
const INSPECT_CLIENT_SUFFIX: &str = " inspect";

pub fn handle_machine(command: &MachineCommands) -> Result<()> {
    match command {
        MachineCommands::Serve {
            owner: ServeCommands::Local,
        } => std::process::exit(local::serve(Path::new(MACHINE_CONFIG_PATH)).code()),
        MachineCommands::Serve {
            owner: ServeCommands::Openrgb,
        } => match owner::serve(Path::new(MACHINE_CONFIG_PATH))? {
            OwnerExit::Stopped => Ok(()),
            // The owner logged why; its status is what systemd acts on.
            exit => std::process::exit(exit.code()),
        },
        MachineCommands::Status => {
            let report = status_report(Path::new(MACHINE_CONFIG_PATH), OwnerUnit::status_path);
            print!("{}", report.text);
            match report.problems.len() {
                0 => Ok(()),
                n => Err(VogixError::Generic(format!(
                    "the machine surfaces have {n} problem{}",
                    if n == 1 { "" } else { "s" }
                ))),
            }
        }
        MachineCommands::Inspect {
            json,
            capture,
            max_protocol,
        } => handle_inspect(*json, capture.as_deref(), *max_protocol),
        MachineCommands::Validate {
            path,
            palette: false,
        } => {
            let path = path.as_deref().unwrap_or(Path::new(MACHINE_CONFIG_PATH));
            let config = MachineConfig::load(path).map_err(config_error)?;
            print!("{}", describe_config(path, &config));
            Ok(())
        }
        MachineCommands::Validate {
            path: Some(path),
            palette: true,
        } => {
            let palette = MachinePalette::load_file(path).map_err(palette_error)?;
            print!("{}", describe_palette(path, &palette, None));
            Ok(())
        }
        MachineCommands::Validate {
            path: None,
            palette: true,
        } => {
            let config =
                MachineConfig::load(Path::new(MACHINE_CONFIG_PATH)).map_err(config_error)?;
            let path = config.drop_zone.as_path().join(PALETTE_FILE);
            let palette = MachinePalette::load_file(&path).map_err(palette_error)?;
            print!("{}", describe_palette(&path, &palette, Some(&config)));
            Ok(())
        }
    }
}

fn config_error(e: ConfigError) -> VogixError {
    VogixError::Config(e.to_string())
}

fn palette_error(e: PaletteError) -> VogixError {
    VogixError::Config(e.to_string())
}

/// What `vogix machine status` prints, and the problems that make it exit 1.
#[derive(Debug, Default)]
struct StatusReport {
    text: String,
    problems: Vec<String>,
}

/// The machine surfaces' state: the config at `config_path`, the palette
/// published into its drop zone, and the status file (at
/// `status_path(unit)`) of every owner unit the config declares.
fn status_report(config_path: &Path, status_path: impl Fn(OwnerUnit) -> PathBuf) -> StatusReport {
    let mut report = StatusReport::default();
    let out = &mut report.text;
    let config = match MachineConfig::load(config_path) {
        Ok(config) => config,
        Err(ConfigError::Absent { path }) => {
            let _ = writeln!(
                out,
                "machine surfaces: not configured ({} does not exist)",
                path.display()
            );
            return report;
        }
        Err(e) => {
            report.problems.push(e.to_string());
            let _ = writeln!(report.text, "machine config: {e}");
            return with_problem_list(report);
        }
    };

    let zone = config.drop_zone.as_path();
    let _ = writeln!(
        out,
        "owner:    {} (drop zone {})",
        config.owner,
        zone.display()
    );
    match MachinePalette::load_from_zone(zone) {
        Ok(palette) => {
            let _ = writeln!(
                out,
                "palette:  {} {} {}",
                palette.theme.scheme, palette.theme.name, palette.theme.variant
            );
        }
        Err(PaletteError::Absent { .. }) => {
            let _ = writeln!(out, "palette:  none published yet");
        }
        Err(e) => {
            let _ = writeln!(out, "palette:  rejected: {e}");
            report
                .problems
                .push(format!("the published palette is rejected: {e}"));
        }
    }

    let declared: Vec<OwnerUnit> = OwnerUnit::ALL
        .into_iter()
        .filter(|unit| unit.is_declared_by(&config))
        .collect();
    if declared.is_empty() {
        let _ = writeln!(out, "no owner unit: the config declares no surface");
    }
    for unit in declared {
        let path = status_path(unit);
        match StatusFile::read(&path) {
            Ok(status) => {
                describe_status(&mut report.text, unit, &status);
                report.problems.extend(
                    status
                        .problems()
                        .into_iter()
                        .map(|p| format!("{}: {p}", unit.unit_name())),
                );
            }
            Err(StatusError::Absent { path }) => {
                let _ = writeln!(
                    report.text,
                    "{}: not running ({} does not exist)",
                    unit.unit_name(),
                    path.display()
                );
                report
                    .problems
                    .push(format!("{} is not running", unit.unit_name()));
            }
            Err(e) => {
                let _ = writeln!(report.text, "{}: {e}", unit.unit_name());
                report
                    .problems
                    .push(format!("{}: unreadable status: {e}", unit.unit_name()));
            }
        }
    }
    with_problem_list(report)
}

fn with_problem_list(mut report: StatusReport) -> StatusReport {
    if !report.problems.is_empty() {
        let _ = writeln!(report.text, "problems:");
        for problem in &report.problems {
            let _ = writeln!(report.text, "  - {problem}");
        }
    }
    report
}

fn describe_status(out: &mut String, unit: OwnerUnit, status: &StatusFile) {
    let _ = write!(out, "{}: {}", unit.unit_name(), status.phase);
    match &status.detail {
        Some(detail) => {
            let _ = writeln!(out, " ({detail})");
        }
        None => {
            let _ = writeln!(out);
        }
    }
    if let Some(theme) = &status.theme {
        let _ = writeln!(
            out,
            "  applied:  {} {} {}",
            theme.scheme, theme.name, theme.variant
        );
    }
    if let Some(protocol) = status.protocol {
        let _ = write!(out, "  server:   protocol {protocol}");
        if let Some(name) = &status.server_name {
            let _ = write!(out, ", {name}");
        }
        if let Some(count) = status.controller_count {
            let _ = write!(out, ", {count} controllers");
        }
        let _ = writeln!(out);
    }
    if let Some(console) = &status.console {
        describe_surface(out, "console", console);
    }
    for (name, device) in &status.devices {
        describe_surface(out, name.as_str(), device);
    }
}

fn describe_surface(out: &mut String, name: &str, surface: &SurfaceStatus) {
    let _ = write!(out, "  {name}: {}", surface.state);
    if let Some(n) = surface.controllers {
        let _ = write!(out, " ({n} controller{})", if n == 1 { "" } else { "s" });
    }
    if let Some(detail) = &surface.detail {
        let _ = write!(out, " — {detail}");
    }
    let _ = writeln!(out);
}

fn handle_inspect(json: bool, capture: Option<&Path>, max_protocol: Option<u32>) -> Result<()> {
    let config_path = Path::new(MACHINE_CONFIG_PATH);
    let config = MachineConfig::load(config_path).map_err(|e| VogixError::Config(e.to_string()))?;
    let Some(endpoint) = config.openrgb else {
        return Err(VogixError::Config(format!(
            "{} declares no OpenRGB endpoint (openrgb is null)",
            config_path.display()
        )));
    };
    let max_protocol = match max_protocol {
        Some(number) => ProtocolVersion::try_from(number)
            .map_err(|e| VogixError::Config(format!("--max-protocol: {e}")))?,
        None => endpoint.max_protocol,
    };
    let client_name = ClientName::new(format!("{}{INSPECT_CLIENT_SUFFIX}", endpoint.client_name))
        .map_err(|e| VogixError::Config(e.to_string()))?;
    let snapshot = inspect::snapshot(
        endpoint.host,
        endpoint.port,
        MirrorConfig {
            max_protocol,
            client_name,
        },
    )
    .map_err(|e| VogixError::Generic(e.to_string()))?;
    if let Some(dir) = capture {
        let manifest = inspect::write_capture(dir, &snapshot)?;
        eprintln!(
            "captured {} controller payload(s) at protocol {}: {}",
            snapshot.controllers.len(),
            snapshot.protocol,
            manifest.display()
        );
    }
    if json {
        print!("{}", inspect::render_json(&snapshot));
    } else {
        print!(
            "{}",
            inspect::render_text(&snapshot, endpoint.host, endpoint.port)
        );
    }
    Ok(())
}

fn describe_config(path: &Path, config: &MachineConfig) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "{}: valid machine config (schema 1)", path.display());
    let _ = writeln!(out, "  owner:     {}", config.owner);
    let _ = writeln!(out, "  drop zone: {}", config.drop_zone.as_path().display());
    let _ = writeln!(
        out,
        "  console:   {}",
        if config.console.enable { "on" } else { "off" }
    );
    match &config.openrgb {
        Some(endpoint) => {
            let _ = writeln!(
                out,
                "  openrgb:   {}:{}, protocol up to {}, client name {:?}",
                endpoint.host,
                endpoint.port,
                endpoint.max_protocol.number(),
                endpoint.client_name.as_str()
            );
        }
        None => {
            let _ = writeln!(out, "  openrgb:   none");
        }
    }
    let units: Vec<&str> = OwnerUnit::ALL
        .into_iter()
        .filter(|unit| unit.is_declared_by(config))
        .map(OwnerUnit::unit_name)
        .collect();
    let _ = writeln!(
        out,
        "  owners:    {}",
        if units.is_empty() {
            "none".to_string()
        } else {
            units.join(", ")
        }
    );
    let _ = writeln!(out, "  devices:   {}", config.devices.len());
    for (name, spec) in &config.devices {
        let how = match &spec.provider {
            Provider::Openrgb(device) => format!(
                "openrgb, name contains {:?}, mode {:?}",
                device.name_contains.as_str(),
                device.mode.as_str()
            ),
            Provider::Command(device) => {
                let mut how = format!(
                    "command {} with {} argument(s)",
                    device.argv.program(),
                    device.argv.args().len()
                );
                if let Some(hidraw) = device.hotplug.hidraw {
                    let _ = write!(how, ", re-run on hidraw {hidraw}");
                }
                how
            }
        };
        let _ = writeln!(out, "    {name}: slot {}, {how}", spec.slot);
    }
    out
}

/// A palette summary; with the machine config, also the colour each
/// declared device resolves to in it.
fn describe_palette(
    path: &Path,
    palette: &MachinePalette,
    config: Option<&MachineConfig>,
) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "{}: valid machine palette (schema 1)", path.display());
    let _ = writeln!(
        out,
        "  theme:   {} {} {}",
        palette.theme.scheme, palette.theme.name, palette.theme.variant
    );
    let _ = writeln!(out, "  slots:   {}", palette.slots.len());
    let console = match (&palette.console, config) {
        (None, _) => "none (the VT palette is left alone)",
        (Some(_), Some(config)) if !config.console.enable => {
            "16 colours (not applied: the config's console is off)"
        }
        (Some(_), _) => "16 colours",
    };
    let _ = writeln!(out, "  console: {console}");
    if let Some(config) = config {
        for (name, spec) in &config.devices {
            match palette.slot(&spec.slot) {
                Some(colour) => {
                    let _ = writeln!(out, "    {name}: {} = {colour}", spec.slot);
                }
                None => {
                    let _ = writeln!(
                        out,
                        "    {name}: {} is not in this palette; the device waits",
                        spec.slot
                    );
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fsutil;
    use crate::machine::status::{Phase, SurfaceState};

    const CONFIG: &str = r#"{
        "schema": 1, "owner": "t", "dropZone": "ZONE",
        "console": { "enable": true },
        "openrgb": { "host": "127.0.0.1", "port": 6742, "maxProtocol": 6, "clientName": "vogix" },
        "devices": {
            "dram-rgb": { "slot": "base01",
                "provider": { "openrgb": { "nameContains": "ENE DRAM", "mode": "Static" } } },
            "kraken-ring": { "slot": "base0Z",
                "provider": { "command": { "argv": ["/bin/liquidctl", "{{color}}"],
                    "hotplug": { "hidraw": { "vendorId": "1e71", "productId": "3012" } } } } }
        }
    }"#;

    const PALETTE: &str = r##"{
        "schema": 1,
        "theme": { "scheme": "vogix16", "name": "nordic", "variant": "dark" },
        "slots": { "base01": "#3b4252" },
        "console": null
    }"##;

    /// A machine config, a drop zone and a runtime directory per owner.
    struct Host {
        dir: tempfile::TempDir,
    }

    impl Host {
        fn new(config: &str) -> Self {
            let dir = tempfile::tempdir().unwrap();
            let zone = dir.path().join("zone");
            std::fs::create_dir(&zone).unwrap();
            let json = config.replace("ZONE", &zone.display().to_string());
            std::fs::write(dir.path().join("machine.json"), json).unwrap();
            Self { dir }
        }

        fn config(&self) -> PathBuf {
            self.dir.path().join("machine.json")
        }

        fn publish(&self, palette: &str) {
            fsutil::write_atomic(
                &self.dir.path().join("zone").join(PALETTE_FILE),
                palette.as_bytes(),
                0o644,
            )
            .unwrap();
        }

        fn status_dir(&self, unit: OwnerUnit) -> PathBuf {
            self.dir.path().join(format!("{unit:?}"))
        }

        fn write_status(&self, status: &StatusFile) {
            let dir = self.status_dir(status.owner);
            std::fs::create_dir_all(&dir).unwrap();
            status.write(&dir).unwrap();
        }

        fn report(&self) -> StatusReport {
            status_report(&self.config(), |unit| {
                self.status_dir(unit)
                    .join(crate::machine::status::STATUS_FILE)
            })
        }
    }

    fn ready(owner: OwnerUnit) -> StatusFile {
        let mut status = StatusFile::new(owner, Phase::Ready);
        match owner {
            OwnerUnit::Openrgb => {
                status.protocol = Some(6);
                status.devices.insert(
                    "dram-rgb".parse().unwrap(),
                    SurfaceStatus {
                        state: SurfaceState::Confirmed,
                        controllers: Some(2),
                        detail: None,
                    },
                );
            }
            OwnerUnit::Local => {
                status.console = Some(SurfaceStatus::new(SurfaceState::Confirmed));
                status.devices.insert(
                    "kraken-ring".parse().unwrap(),
                    SurfaceStatus::new(SurfaceState::Confirmed),
                );
            }
        }
        status
    }

    #[test]
    fn healthy_owners_report_no_problem() {
        let host = Host::new(CONFIG);
        host.publish(PALETTE);
        host.write_status(&ready(OwnerUnit::Openrgb));
        host.write_status(&ready(OwnerUnit::Local));
        let report = host.report();
        assert!(report.problems.is_empty(), "{}", report.text);
        assert!(report.text.contains("owner:    t"), "{}", report.text);
        assert!(
            report.text.contains("palette:  vogix16 nordic dark"),
            "{}",
            report.text
        );
        assert!(
            report
                .text
                .contains("  dram-rgb: confirmed (2 controllers)"),
            "{}",
            report.text
        );
        assert!(
            report
                .text
                .contains("vogix-machine.service: ready\n  console: confirmed"),
            "{}",
            report.text
        );
    }

    #[test]
    fn a_declared_owner_without_status_is_a_problem() {
        let host = Host::new(CONFIG);
        host.write_status(&ready(OwnerUnit::Local));
        let report = host.report();
        assert_eq!(
            report.problems,
            ["vogix-openrgb.service is not running"],
            "{}",
            report.text
        );
        assert!(report.text.contains("palette:  none published yet"));
    }

    #[test]
    fn an_owner_the_config_does_not_declare_is_not_expected() {
        let host = Host::new(&CONFIG.replace(
            r#""dram-rgb": { "slot": "base01",
                "provider": { "openrgb": { "nameContains": "ENE DRAM", "mode": "Static" } } },"#,
            "",
        ));
        host.write_status(&ready(OwnerUnit::Local));
        let report = host.report();
        assert!(report.problems.is_empty(), "{}", report.text);
        assert!(!report.text.contains("vogix-openrgb"), "{}", report.text);
    }

    #[test]
    fn faults_errors_and_a_rejected_palette_are_problems() {
        let host = Host::new(CONFIG);
        host.publish(r#"{"schema": 1}"#);
        let mut faulted = StatusFile::new(OwnerUnit::Openrgb, Phase::Faulted);
        faulted.detail = Some("bad magic".into());
        host.write_status(&faulted);
        let mut local = ready(OwnerUnit::Local);
        local.devices.insert(
            "kraken-ring".parse().unwrap(),
            SurfaceStatus {
                state: SurfaceState::Error,
                controllers: None,
                detail: Some("exit status: 1".into()),
            },
        );
        host.write_status(&local);
        let report = host.report();
        assert_eq!(report.problems.len(), 3, "{}", report.text);
        assert!(report.problems[0].starts_with("the published palette is rejected"));
        assert_eq!(
            report.problems[1],
            "vogix-openrgb.service: faulted: bad magic"
        );
        assert_eq!(
            report.problems[2],
            "vogix-machine.service: kraken-ring: error: exit status: 1"
        );
        assert!(report.text.contains("problems:\n  - "), "{}", report.text);
    }

    #[test]
    fn no_machine_config_is_not_configured_and_no_problem() {
        let report = status_report(
            Path::new("/nonexistent/machine.json"),
            OwnerUnit::status_path,
        );
        assert!(report.problems.is_empty());
        assert!(report.text.starts_with("machine surfaces: not configured"));
    }

    #[test]
    fn an_invalid_machine_config_is_a_problem() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("machine.json");
        std::fs::write(&path, br#"{"schema": 2}"#).unwrap();
        let report = status_report(&path, OwnerUnit::status_path);
        assert_eq!(report.problems.len(), 1);
        assert!(report.problems[0].contains("schema 2 is not supported"));
    }

    #[test]
    fn the_config_summary_names_every_device_and_its_owner_units() {
        let config =
            MachineConfig::from_json(CONFIG.replace("ZONE", "/var/lib/vogix/machine").as_bytes())
                .unwrap();
        let text = describe_config(Path::new("/etc/vogix/machine.json"), &config);
        assert!(text.contains("owner:     t"), "{text}");
        assert!(text.contains("127.0.0.1:6742, protocol up to 6"), "{text}");
        assert!(
            text.contains("owners:    vogix-openrgb.service, vogix-machine.service"),
            "{text}"
        );
        assert!(
            text.contains(
                "dram-rgb: slot base01, openrgb, name contains \"ENE DRAM\", mode \"Static\""
            ),
            "{text}"
        );
        assert!(
            text.contains("kraken-ring: slot base0Z, command /bin/liquidctl with 1 argument(s), re-run on hidraw 1e71:3012"),
            "{text}"
        );
    }

    #[test]
    fn a_palette_checked_against_the_config_resolves_each_device() {
        let host = Host::new(CONFIG);
        host.publish(PALETTE);
        let config = MachineConfig::load(&host.config()).unwrap();
        let path = host.dir.path().join("zone").join(PALETTE_FILE);
        let palette = MachinePalette::load_file(&path).unwrap();
        let text = describe_palette(&path, &palette, Some(&config));
        assert!(text.contains("dram-rgb: base01 = #3b4252"), "{text}");
        assert!(
            text.contains("kraken-ring: base0Z is not in this palette; the device waits"),
            "{text}"
        );
        assert!(text.contains("console: none (the VT palette is left alone)"));
        let alone = describe_palette(&path, &palette, None);
        assert!(!alone.contains("dram-rgb"), "{alone}");
    }

    #[test]
    fn a_rejected_file_is_an_error_naming_it() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("machine.json");
        std::fs::write(&path, br#"{"schema": 2}"#).unwrap();
        let err = handle_machine(&MachineCommands::Validate {
            path: Some(path.clone()),
            palette: false,
        })
        .unwrap_err()
        .to_string();
        assert!(err.contains(&path.display().to_string()), "{err}");
        assert!(err.contains("schema 2 is not supported"), "{err}");

        let err = handle_machine(&MachineCommands::Validate {
            path: Some(dir.path().join("palette.json")),
            palette: true,
        })
        .unwrap_err()
        .to_string();
        assert!(err.contains("does not exist"), "{err}");
    }
}

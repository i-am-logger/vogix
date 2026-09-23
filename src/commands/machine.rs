//! `vogix machine` — the command-line side of the machine surfaces.
//!
//! `validate` runs the loaders the machine owners use on a given file, so a
//! rendered `/etc/vogix/machine.json` or a published `palette.json` is
//! accepted or rejected here exactly as the owners would.

use crate::cli::MachineCommands;
use crate::errors::{Result, VogixError};
use crate::machine::config::{MachineConfig, Provider};
use crate::machine::palette::MachinePalette;
use std::fmt::Write as _;
use std::path::Path;

pub fn handle_machine(command: &MachineCommands) -> Result<()> {
    match command {
        MachineCommands::Validate {
            path,
            palette: false,
        } => {
            let config =
                MachineConfig::load(path).map_err(|e| VogixError::Config(e.to_string()))?;
            print!("{}", describe_config(path, &config));
            Ok(())
        }
        MachineCommands::Validate {
            path,
            palette: true,
        } => {
            let palette =
                MachinePalette::load_file(path).map_err(|e| VogixError::Config(e.to_string()))?;
            print!("{}", describe_palette(path, &palette));
            Ok(())
        }
    }
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

fn describe_palette(path: &Path, palette: &MachinePalette) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "{}: valid machine palette (schema 1)", path.display());
    let _ = writeln!(
        out,
        "  theme:   {} {} {}",
        palette.theme.scheme, palette.theme.name, palette.theme.variant
    );
    let _ = writeln!(out, "  slots:   {}", palette.slots.len());
    let _ = writeln!(
        out,
        "  console: {}",
        if palette.console.is_some() {
            "16 colours"
        } else {
            "none"
        }
    );
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_config_summary_names_every_device_and_how_it_is_driven() {
        let config = MachineConfig::from_json(
            br#"{
                "schema": 1, "owner": "t", "dropZone": "/var/lib/vogix/machine",
                "console": { "enable": true },
                "openrgb": { "host": "127.0.0.1", "port": 6742, "maxProtocol": 6, "clientName": "vogix" },
                "devices": {
                    "dram-rgb": { "slot": "base01",
                        "provider": { "openrgb": { "nameContains": "ENE DRAM", "mode": "Static" } } },
                    "kraken-ring": { "slot": "base01",
                        "provider": { "command": { "argv": ["/bin/liquidctl", "{{color}}"],
                            "hotplug": { "hidraw": { "vendorId": "1e71", "productId": "3012" } } } } }
                }
            }"#,
        )
        .unwrap();
        let text = describe_config(Path::new("/etc/vogix/machine.json"), &config);
        assert!(text.contains("owner:     t"), "{text}");
        assert!(text.contains("127.0.0.1:6742, protocol up to 6"), "{text}");
        assert!(
            text.contains(
                "dram-rgb: slot base01, openrgb, name contains \"ENE DRAM\", mode \"Static\""
            ),
            "{text}"
        );
        assert!(
            text.contains("kraken-ring: slot base01, command /bin/liquidctl with 1 argument(s), re-run on hidraw 1e71:3012"),
            "{text}"
        );
    }

    #[test]
    fn a_rejected_file_is_an_error_naming_it() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("machine.json");
        std::fs::write(&path, br#"{"schema": 2}"#).unwrap();
        let err = handle_machine(&MachineCommands::Validate {
            path: path.clone(),
            palette: false,
        })
        .unwrap_err()
        .to_string();
        assert!(err.contains(&path.display().to_string()), "{err}");
        assert!(err.contains("schema 2 is not supported"), "{err}");

        let err = handle_machine(&MachineCommands::Validate {
            path: dir.path().join("palette.json"),
            palette: true,
        })
        .unwrap_err()
        .to_string();
        assert!(err.contains("does not exist"), "{err}");
    }
}

//! `vogix machine inspect`: what the OpenRGB server serves, read-only.
//!
//! A second SDK client beside the owner. It negotiates, enumerates the
//! controller list once through a [`Mirror`] — the session's read side, whose
//! write paths are private to the session layer, so nothing on this path can
//! send `UPDATELEDS` or `UPDATEMODE` — then closes once every queued request is
//! written and, at protocol 6, acknowledged, and reports what it saw. With
//! `--capture` it also writes each controller's raw `REQUEST_CONTROLLER_DATA`
//! payload and a manifest: the fixtures vogix's decoder tests load.

use super::connection::{self, Peer, READ_CHUNK};
use super::model::{ColorMode, ControllerDescription, ModeDescription, ModeFlags, ProtocolVersion};
use super::session::mirror::{Controller, Fault};
use super::session::{Mirror, MirrorConfig, MirrorEvent};
use super::wire::{RgbColor, WireString};
use crate::fsutil;
use crate::machine::reactor::{Interest, PollSet};
use crate::machine::types::SchemaV1;
use serde::{Deserialize, Serialize};
use std::fmt::Write as _;
use std::io::{self, Read, Write};
use std::net::Ipv4Addr;
use std::os::fd::AsFd;
use std::path::{Path, PathBuf};

/// The controller list as the server described it once.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snapshot {
    /// The highest protocol the server offered.
    pub server_max: u32,
    pub protocol: ProtocolVersion,
    pub server_name: Option<String>,
    /// In server list order.
    pub controllers: Vec<Controller>,
}

#[derive(Debug, thiserror::Error)]
pub enum InspectError {
    #[error("cannot connect to OpenRGB at {host}:{port}: {source}")]
    Connect {
        host: Ipv4Addr,
        port: u16,
        source: io::Error,
    },
    #[error("OpenRGB closed the connection before it listed its controllers{}", .error.as_ref().map(|e| format!(": {e}")).unwrap_or_default())]
    Closed { error: Option<io::Error> },
    #[error(transparent)]
    Fault(Fault),
    #[error("waiting for OpenRGB: {0}")]
    Wait(io::Error),
}

/// Connect to `host:port`, list the controllers once, and close.
pub fn snapshot(host: Ipv4Addr, port: u16, config: MirrorConfig) -> Result<Snapshot, InspectError> {
    let mut stream = connection::connect(host, port).map_err(|source| InspectError::Connect {
        host,
        port,
        source,
    })?;
    drive(&mut stream, Mirror::new(config))
}

/// Drive `mirror` over a connected, non-blocking `stream` until the first
/// controller list is committed and the connection has drained.
pub fn drive<S: Read + Write + AsFd>(
    stream: &mut S,
    mirror: Mirror,
) -> Result<Snapshot, InspectError> {
    let mut inspector = Inspector::new(mirror);
    loop {
        let interest = match inspector.pump(stream) {
            Step::Done(outcome) => return outcome,
            Step::Wait(interest) => interest,
        };
        let mut set = PollSet::new();
        set.add(stream.as_fd(), interest, ());
        set.wait().map_err(InspectError::Wait)?;
    }
}

/// What [`Inspector::pump`] needs next.
#[derive(Debug)]
enum Step {
    /// Wait for the socket to become ready for this.
    Wait(Interest),
    Done(Result<Snapshot, InspectError>),
}

/// inspect's progress on one connection.
struct Inspector {
    mirror: Mirror,
    buf: Vec<u8>,
    server_max: Option<u32>,
    /// The first committed list; the connection then drains and closes.
    listed: Option<Snapshot>,
}

impl Inspector {
    fn new(mirror: Mirror) -> Self {
        Self {
            mirror,
            buf: vec![0u8; READ_CHUNK],
            server_max: None,
            listed: None,
        }
    }

    /// The list, or why there is none, once the connection is gone.
    fn closed(&mut self, error: Option<io::Error>) -> Step {
        Step::Done(match self.listed.take() {
            // The list is complete; only the drain was cut short.
            Some(snapshot) => Ok(snapshot),
            None => Err(InspectError::Closed { error }),
        })
    }

    /// Read what arrived, act on it, write what is queued.
    fn pump<S: Read + Write>(&mut self, stream: &mut S) -> Step {
        let Self { mirror, buf, .. } = self;
        let peer = connection::read_available(stream, buf, |bytes| mirror.receive(bytes));
        while let Some(event) = self.mirror.pop_event() {
            match event {
                MirrorEvent::Negotiated { server_max, .. } => self.server_max = Some(server_max),
                MirrorEvent::ListCommitted { .. } if self.listed.is_none() => {
                    let (Some(server_max), Some(protocol)) =
                        (self.server_max, self.mirror.protocol())
                    else {
                        return Step::Done(Err(InspectError::Fault(Fault::Unexpected {
                            packet: super::wire::PacketId::RequestControllerCount,
                            dev_id: 0,
                            detail: "a controller list was committed before the version was agreed",
                        })));
                    };
                    self.listed = Some(Snapshot {
                        server_max,
                        protocol,
                        server_name: self
                            .mirror
                            .server_name()
                            .map(|name| name.to_string_lossy().into_owned()),
                        controllers: self.mirror.controllers().cloned().collect(),
                    });
                    self.mirror.begin_shutdown();
                }
                MirrorEvent::Faulted(fault) => return Step::Done(Err(InspectError::Fault(fault))),
                _ => {}
            }
        }
        if let Peer::Closed { error } = peer {
            return self.closed(error);
        }
        match connection::write_available(stream, self.mirror.pending_output()) {
            Ok(written) => self.mirror.consume_output(written),
            Err(e) => return self.closed(Some(e)),
        }
        if self.listed.is_some() && self.mirror.should_close() {
            return self.closed(None);
        }
        Step::Wait(if self.mirror.pending_output().is_empty() {
            Interest::Read
        } else {
            Interest::ReadWrite
        })
    }
}

/// The `--json` view.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SnapshotView<'a> {
    server_name: Option<&'a str>,
    server_max: u32,
    protocol: ProtocolVersion,
    controllers: Vec<ControllerView<'a>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ControllerView<'a> {
    index: usize,
    dev_id: u32,
    display_name: &'a WireString,
    description: &'a ControllerDescription,
}

/// The snapshot as pretty JSON: the decoded descriptions of every controller.
pub fn render_json(snapshot: &Snapshot) -> String {
    let view = SnapshotView {
        server_name: snapshot.server_name.as_deref(),
        server_max: snapshot.server_max,
        protocol: snapshot.protocol,
        controllers: snapshot
            .controllers
            .iter()
            .enumerate()
            .map(|(index, controller)| ControllerView {
                index,
                dev_id: controller.dev_id,
                display_name: controller.description.display_name(),
                description: &controller.description,
            })
            .collect(),
    };
    let mut json = serde_json::to_string_pretty(&view).expect("a snapshot always serializes");
    json.push('\n');
    json
}

/// Runs of equal colours: `#3b4252 x5, #000000 x3`.
fn colour_runs(colours: &[RgbColor]) -> String {
    let mut runs: Vec<(RgbColor, usize)> = Vec::new();
    for colour in colours {
        match runs.last_mut() {
            Some((last, count)) if last == colour => *count += 1,
            _ => runs.push((*colour, 1)),
        }
    }
    runs.iter()
        .map(|(colour, count)| {
            if *count == 1 {
                colour.to_string()
            } else {
                format!("{colour} x{count}")
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn describe_mode(out: &mut String, mode: &ModeDescription, active: bool) {
    let _ = write!(
        out,
        "    {} {}: ",
        if active { "*" } else { " " },
        mode.name
    );
    let colours = match mode.color_mode {
        ColorMode::PerLed => "per-LED colours".to_owned(),
        ColorMode::ModeSpecific => format!(
            "mode colours [{}] ({}..{})",
            colour_runs(&mode.colors),
            mode.colors_min,
            mode.colors_max
        ),
        other => format!("{other} colours"),
    };
    out.push_str(&colours);
    if mode.flags.contains(ModeFlags::HAS_BRIGHTNESS) {
        let _ = write!(
            out,
            ", brightness {} ({}..{})",
            mode.brightness, mode.brightness_min, mode.brightness_max
        );
    }
    if mode.flags.contains(ModeFlags::HAS_SPEED) {
        let _ = write!(
            out,
            ", speed {} ({}..{})",
            mode.speed, mode.speed_min, mode.speed_max
        );
    }
    out.push('\n');
}

/// The snapshot as text: each controller's name, identity, modes (the active
/// one starred), zones and colours.
pub fn render_text(snapshot: &Snapshot, host: Ipv4Addr, port: u16) -> String {
    let mut out = String::new();
    let _ = writeln!(
        out,
        "OpenRGB{} at {host}:{port}: SDK protocol {} (the server offers {})",
        snapshot
            .server_name
            .as_deref()
            .map_or_else(String::new, |name| format!(" {name:?}")),
        snapshot.protocol,
        snapshot.server_max
    );
    let _ = writeln!(out, "{} controller(s)", snapshot.controllers.len());
    for (index, controller) in snapshot.controllers.iter().enumerate() {
        let d = &controller.description;
        let _ = writeln!(out);
        let address = match snapshot.protocol {
            ProtocolVersion::V5 => format!("index {}", controller.dev_id),
            ProtocolVersion::V6 => format!("id {}", controller.dev_id),
        };
        let _ = writeln!(out, "[{index}] {} ({address})", d.display_name());
        let kind = d
            .device_type
            .name()
            .map_or_else(|| format!("type {}", d.device_type.0), str::to_owned);
        let _ = writeln!(
            out,
            "    {kind}; vendor {:?}; location {:?}",
            d.vendor.to_string_lossy(),
            d.location.to_string_lossy()
        );
        let _ = writeln!(out, "    modes:");
        for (mode_index, mode) in d.modes.iter().enumerate() {
            let active = usize::try_from(d.active_mode).is_ok_and(|active| active == mode_index);
            describe_mode(&mut out, mode, active);
        }
        if !d.zones.is_empty() {
            let zones = d
                .zones
                .iter()
                .map(|zone| {
                    let plural = if zone.leds_count == 1 { "" } else { "s" };
                    format!("{} ({} LED{plural})", zone.name, zone.leds_count)
                })
                .collect::<Vec<_>>()
                .join(", ");
            let _ = writeln!(out, "    zones: {zones}");
        }
        let _ = writeln!(
            out,
            "    colours: {}",
            if d.colors.is_empty() {
                "none".to_owned()
            } else {
                colour_runs(&d.colors)
            }
        );
    }
    out
}

/// `manifest.json` of a capture directory.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CaptureManifest {
    pub schema: SchemaV1,
    /// The protocol every payload was serialised at.
    pub protocol: ProtocolVersion,
    pub server_max: u32,
    pub server_name: Option<String>,
    pub controllers: Vec<CapturedController>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CapturedController {
    /// Position in the server's list.
    pub index: usize,
    pub dev_id: u32,
    pub display_name: String,
    /// The payload file, relative to the manifest.
    pub file: String,
    pub bytes: usize,
}

pub const MANIFEST_FILE: &str = "manifest.json";

/// Write each controller's raw `REQUEST_CONTROLLER_DATA` payload as
/// `controller-NN.bin` and a `manifest.json` into `dir` (created if needed);
/// returns the manifest's path.
pub fn write_capture(dir: &Path, snapshot: &Snapshot) -> io::Result<PathBuf> {
    std::fs::create_dir_all(dir)?;
    let mut controllers = Vec::with_capacity(snapshot.controllers.len());
    for (index, controller) in snapshot.controllers.iter().enumerate() {
        let file = format!("controller-{index:02}.bin");
        fsutil::write_atomic(&dir.join(&file), &controller.raw, 0o644)?;
        controllers.push(CapturedController {
            index,
            dev_id: controller.dev_id,
            display_name: controller
                .description
                .display_name()
                .to_string_lossy()
                .into_owned(),
            file,
            bytes: controller.raw.len(),
        });
    }
    let manifest = CaptureManifest {
        schema: SchemaV1,
        protocol: snapshot.protocol,
        server_max: snapshot.server_max,
        server_name: snapshot.server_name.clone(),
        controllers,
    };
    let mut json = serde_json::to_vec_pretty(&manifest).expect("a manifest always serializes");
    json.push(b'\n');
    let path = dir.join(MANIFEST_FILE);
    fsutil::write_atomic(&path, &json, 0o644)?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    //! State-machine tests of inspect's own logic over a socket pair whose far
    //! end plays a scripted byte stream (`testkit::server`); they prove what
    //! inspect sends and how it closes, not protocol compatibility, which the
    //! real-server VM check establishes.

    use super::super::codec;
    use super::super::model::{AckStatus, ClientName};
    use super::super::testkit::{self, server};
    use super::super::wire::{Framer, PacketId};
    use super::*;
    use std::os::unix::net::UnixStream;
    use std::thread;

    fn config(max_protocol: ProtocolVersion) -> MirrorConfig {
        MirrorConfig {
            max_protocol,
            client_name: ClientName::new("vogix inspect").unwrap(),
        }
    }

    /// Run inspect against `script`, which the peer writes at once (then shuts
    /// its sending side when `then_close`); returns the result and every frame
    /// inspect sent before it closed.
    fn run(
        max_protocol: ProtocolVersion,
        script: Vec<Vec<u8>>,
        then_close: bool,
    ) -> (Result<Snapshot, InspectError>, Vec<(PacketId, u32)>) {
        let (mut client, mut peer) = UnixStream::pair().unwrap();
        client.set_nonblocking(true).unwrap();
        let reader = thread::spawn(move || {
            for frame in &script {
                peer.write_all(frame).unwrap();
            }
            if then_close {
                peer.shutdown(std::net::Shutdown::Write).unwrap();
            }
            let mut sent = Vec::new();
            peer.read_to_end(&mut sent).unwrap();
            sent
        });
        let result = drive(&mut client, Mirror::new(config(max_protocol)));
        drop(client);
        let sent = reader.join().unwrap();
        let mut framer = Framer::new();
        framer.push(&sent);
        let mut frames = Vec::new();
        while let Some(frame) = framer.next_frame().unwrap() {
            frames.push((frame.pkt_id, frame.dev_id));
        }
        assert_eq!(framer.buffered(), 0);
        (result, frames)
    }

    fn v6_script() -> Vec<Vec<u8>> {
        let dram = testkit::ene_dram(ProtocolVersion::V6, 0);
        let govee = testkit::govee(ProtocolVersion::V6);
        vec![
            server::version_reply(6),
            server::server_name("OpenRGB"),
            server::ok(0, PacketId::RequestProtocolVersion),
            server::count_v6(&[7, 9]),
            server::ok(0, PacketId::RequestControllerCount),
            server::ok(0, PacketId::SetClientFlags),
            server::ok(0, PacketId::SetClientName),
            server::controller_data(7, &dram, ProtocolVersion::V6),
            server::ok(7, PacketId::RequestControllerData),
            server::controller_data(9, &govee, ProtocolVersion::V6),
            server::ok(9, PacketId::RequestControllerData),
        ]
    }

    /// The packets a read-only client may send.
    fn read_only(frames: &[(PacketId, u32)]) -> bool {
        frames.iter().all(|(packet, _)| {
            matches!(
                packet,
                PacketId::RequestProtocolVersion
                    | PacketId::RequestControllerCount
                    | PacketId::RequestControllerData
                    | PacketId::SetClientFlags
                    | PacketId::SetClientName
            )
        })
    }

    #[test]
    fn inspect_lists_the_controllers_and_never_sends_a_write() {
        let (result, frames) = run(ProtocolVersion::V6, v6_script(), false);
        let snapshot = result.unwrap();
        assert_eq!(snapshot.protocol, ProtocolVersion::V6);
        assert_eq!(snapshot.server_max, 6);
        assert_eq!(snapshot.server_name.as_deref(), Some("OpenRGB"));
        let ids: Vec<u32> = snapshot.controllers.iter().map(|c| c.dev_id).collect();
        assert_eq!(ids, [7, 9]);
        assert_eq!(
            snapshot.controllers[0].raw,
            testkit::controller_data_payload(
                &testkit::ene_dram(ProtocolVersion::V6, 0),
                ProtocolVersion::V6
            ),
            "the raw payload is kept for --capture"
        );

        assert!(read_only(&frames), "{frames:?}");
        assert!(
            !frames
                .iter()
                .any(|(p, _)| matches!(p, PacketId::UpdateLeds | PacketId::UpdateMode))
        );
        assert_eq!(
            frames,
            [
                (PacketId::RequestProtocolVersion, 0),
                (PacketId::RequestControllerCount, 0),
                (PacketId::SetClientFlags, 0),
                (PacketId::SetClientName, 0),
                (PacketId::RequestControllerData, 7),
                (PacketId::RequestControllerData, 9),
            ]
        );
    }

    #[test]
    fn at_protocol_5_inspect_enumerates_by_index_and_stays_read_only() {
        let dram = testkit::ene_dram(ProtocolVersion::V5, 0);
        let script = vec![
            server::version_reply(6),
            server::count_v5(1),
            server::controller_data(0, &dram, ProtocolVersion::V5),
            server::count_v5(1),
        ];
        let (result, frames) = run(ProtocolVersion::V5, script, false);
        let snapshot = result.unwrap();
        assert_eq!(snapshot.protocol, ProtocolVersion::V5);
        assert_eq!(snapshot.server_max, 6);
        assert_eq!(snapshot.controllers.len(), 1);
        assert_eq!(snapshot.controllers[0].description, dram);
        assert!(read_only(&frames), "{frames:?}");
    }

    #[test]
    fn inspect_waits_for_every_acknowledgement_before_it_closes() {
        // An empty list commits on the count reply, while the count, flags
        // and name requests still await their acknowledgements (protocol 6):
        // inspect has its answer but must not close before them.
        let script = [
            server::version_reply(6),
            server::ok(0, PacketId::RequestProtocolVersion),
            server::count_v6(&[]),
        ];
        let acks = [
            server::ok(0, PacketId::RequestControllerCount),
            server::ok(0, PacketId::SetClientFlags),
            server::ok(0, PacketId::SetClientName),
        ];
        // Bytes a peer writes to a socket pair are readable once the write
        // returns, so each pump below sees exactly what was written before it.
        let (mut client, mut peer) = UnixStream::pair().unwrap();
        client.set_nonblocking(true).unwrap();
        let mut inspector = Inspector::new(Mirror::new(config(ProtocolVersion::V6)));
        assert!(matches!(
            inspector.pump(&mut client),
            Step::Wait(Interest::Read)
        ));
        for frame in &script {
            peer.write_all(frame).unwrap();
        }
        assert!(matches!(
            inspector.pump(&mut client),
            Step::Wait(Interest::Read)
        ));
        assert!(inspector.listed.is_some(), "the empty list is committed");

        peer.write_all(&acks[0]).unwrap();
        peer.write_all(&acks[1]).unwrap();
        assert!(
            matches!(inspector.pump(&mut client), Step::Wait(Interest::Read)),
            "inspect closed with an acknowledgement outstanding"
        );
        peer.write_all(&acks[2]).unwrap();
        let Step::Done(outcome) = inspector.pump(&mut client) else {
            panic!("inspect kept the drained connection open");
        };
        assert!(outcome.unwrap().controllers.is_empty());

        drop(client);
        let mut sent = Vec::new();
        peer.read_to_end(&mut sent).unwrap();
        let mut framer = Framer::new();
        framer.push(&sent);
        let mut frames = Vec::new();
        while let Some(frame) = framer.next_frame().unwrap() {
            frames.push(frame.pkt_id);
        }
        assert_eq!(
            frames,
            [
                PacketId::RequestProtocolVersion,
                PacketId::RequestControllerCount,
                PacketId::SetClientFlags,
                PacketId::SetClientName,
            ]
        );
    }

    #[test]
    fn a_close_before_the_list_is_an_error() {
        let (result, _) = run(ProtocolVersion::V6, vec![server::version_reply(6)], true);
        let err = result.unwrap_err();
        assert!(matches!(err, InspectError::Closed { .. }), "{err:?}");
        assert!(err.to_string().contains("before it listed its controllers"));
    }

    #[test]
    fn a_protocol_fault_is_the_error() {
        // A count reply ahead of the version reply: a protocol 0 server.
        let (result, _) = run(ProtocolVersion::V6, vec![server::count_v5(0)], false);
        assert!(matches!(
            result.unwrap_err(),
            InspectError::Fault(Fault::ProtocolZero)
        ));
    }

    #[test]
    fn a_rejected_request_is_not_a_write() {
        // A non-OK acknowledgement of a read request is reported by the
        // mirror, and inspect still lists what it has.
        let mut script = v6_script();
        let last = script.len() - 1;
        script[last] = server::ack(9, PacketId::RequestControllerData, AckStatus::ErrorGeneric);
        let (result, frames) = run(ProtocolVersion::V6, script, false);
        assert_eq!(result.unwrap().controllers.len(), 2);
        assert!(read_only(&frames));
    }

    fn snapshot_of(protocol: ProtocolVersion) -> Snapshot {
        let dram = testkit::ene_dram(protocol, 0);
        let govee = testkit::govee(protocol);
        Snapshot {
            server_max: 6,
            protocol,
            server_name: Some("OpenRGB".into()),
            controllers: vec![
                Controller {
                    dev_id: 7,
                    raw: testkit::controller_data_payload(&dram, protocol),
                    description: dram,
                },
                Controller {
                    dev_id: 9,
                    raw: testkit::controller_data_payload(&govee, protocol),
                    description: govee,
                },
            ],
        }
    }

    #[test]
    fn a_capture_holds_every_raw_payload_and_a_manifest() {
        let dir = tempfile::tempdir().unwrap();
        let snapshot = snapshot_of(ProtocolVersion::V6);
        let manifest_path = write_capture(&dir.path().join("v6"), &snapshot).unwrap();
        let manifest: CaptureManifest =
            serde_json::from_slice(&std::fs::read(&manifest_path).unwrap()).unwrap();
        assert_eq!(manifest.protocol, ProtocolVersion::V6);
        assert_eq!(manifest.controllers.len(), 2);
        for (captured, controller) in manifest.controllers.iter().zip(&snapshot.controllers) {
            let raw = std::fs::read(dir.path().join("v6").join(&captured.file)).unwrap();
            assert_eq!(raw, controller.raw);
            assert_eq!(captured.bytes, raw.len());
            assert_eq!(captured.dev_id, controller.dev_id);
            assert_eq!(
                codec::decode_controller_data(&raw, manifest.protocol).unwrap(),
                controller.description
            );
        }
        assert_eq!(manifest.controllers[1].file, "controller-01.bin");
    }

    #[test]
    fn text_and_json_name_every_controller_and_its_modes() {
        let snapshot = snapshot_of(ProtocolVersion::V6);
        let text = render_text(&snapshot, Ipv4Addr::LOCALHOST, 6742);
        assert!(
            text.starts_with(
                "OpenRGB \"OpenRGB\" at 127.0.0.1:6742: SDK protocol 6 (the server offers 6)\n"
            ),
            "{text}"
        );
        assert!(text.contains("[0] ENE DRAM (id 7)"), "{text}");
        assert!(text.contains("    * Direct: per-LED colours\n"), "{text}");
        assert!(text.contains("    zones: DRAM (8 LEDs)\n"), "{text}");
        assert!(text.contains("    colours: #000000 x8\n"), "{text}");
        assert!(text.contains("[1] Govee H6199 (id 9)"), "{text}");
        assert!(
            text.contains("      Static: mode colours [#000000] (1..1), brightness 100 (0..100)\n"),
            "{text}"
        );
        assert!(text.contains("    zones: Govee (1 LED)\n"), "{text}");

        let v5 = render_text(&snapshot_of(ProtocolVersion::V5), Ipv4Addr::LOCALHOST, 6742);
        assert!(v5.contains("[0] ENE DRAM (index 7)"), "{v5}");

        let json: serde_json::Value = serde_json::from_str(&render_json(&snapshot)).unwrap();
        assert_eq!(json["protocol"], 6);
        assert_eq!(json["controllers"][0]["devId"], 7);
        assert_eq!(json["controllers"][0]["displayName"], "ENE DRAM");
        assert!(json["controllers"][1]["description"]["modes"].is_array());
    }

    #[test]
    fn colour_runs_compress_equal_neighbours() {
        let a = RgbColor::from_rgb(0x3b, 0x42, 0x52);
        let b = RgbColor::from_rgb(0, 0, 0);
        assert_eq!(
            colour_runs(&[a, a, a, b, a]),
            "#3b4252 x3, #000000, #3b4252"
        );
    }
}

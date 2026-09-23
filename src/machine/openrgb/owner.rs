//! `vogix machine serve openrgb`, the process behind `vogix-openrgb.service`:
//! the one vogix client of the OpenRGB SDK server.
//!
//! It follows the machine owner's published palette. Every OpenRGB device in
//! `/etc/vogix/machine.json` selects controllers by name and names a mode and
//! a palette slot; the slot's colour becomes that device's [`Target`], and the
//! [`Session`] applies it to every controller the device selects and confirms
//! it (protocol 6) or reports it sent (protocol 5).
//!
//! # Events
//!
//! Startup reads the config, starts watching the drop zone, reads
//! `palette.json` if one is published, then connects — a blocking connect to
//! the loopback endpoint, which completes or is refused at once. From then on
//! the owner waits in one `poll(2)` with no timeout over the SDK socket
//! (writability only while bytes are queued), the drop zone's inotify fd and a
//! signalfd:
//!
//! - a replaced `palette.json` is read once, however many replacements
//!   queued, and its colours become the targets;
//! - SIGHUP re-reads the palette and forces a re-apply, writing modes even
//!   where they are already active; on a faulted session it connects anew
//!   once the old connection has drained;
//! - SIGTERM or SIGINT sends `STOPPING=1`, stops requesting, and closes once
//!   every queued byte is written and — at protocol 6 — every outstanding
//!   acknowledgement arrived, then exits 0.
//!
//! Every state change is sent as `STATUS=` and written to `status.json` in the
//! unit's runtime directory.
//!
//! # Exits and faults
//!
//! - 0: stopped by SIGTERM or SIGINT.
//! - 75 (`EX_TEMPFAIL`): OpenRGB refused the connection or closed it.
//!   openrgb.service's own restart and readiness, with `Upholds=`, start the
//!   owner again.
//! - 78 (`EX_CONFIG`): the machine config was rejected or declares no OpenRGB
//!   endpoint, or the drop zone is missing or was removed.
//!
//! A protocol violation the client detects ([`super::session::mirror::Fault`])
//! is deterministic, so it does not exit into a restart loop: the owner closes
//! the connection (after outstanding writes are acknowledged), reports
//! `faulted`, and stays so until SIGHUP, a stop, or a restart of openrgb.service
//! (which stops and restarts the owner through `BindsTo=` and `Upholds=`).

use super::connection::{self, Peer, READ_CHUNK};
use super::model::{ClientName, ProtocolVersion};
use super::select::Selector;
use super::session::mirror::Phase as LinkPhase;
use super::session::{
    ApplyState, DeviceReport, DeviceState, Event, MirrorConfig, MirrorEvent, Session, Target,
};
use super::wire::RgbColor;
use crate::machine::config::{MachineConfig, OpenRgbEndpoint};
use crate::machine::notify::{Notification, Notifier, StatusLine};
use crate::machine::palette::{MachinePalette, PALETTE_FILE, PaletteError};
use crate::machine::reactor::{
    DropZoneWatch, Interest, PollSet, Readiness, Signal, SignalFd, ZoneEvents,
};
use crate::machine::status::{
    self, OwnerUnit, Phase, STATUS_FILE, StatusFile, SurfaceState, SurfaceStatus,
};
use crate::machine::types::{DeviceName, Rgb};
use log::{debug, error, info, trace, warn};
use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::io;
use std::net::TcpStream;
use std::os::fd::AsFd;
use std::path::{Path, PathBuf};

/// How the owner ended; [`OwnerExit::code`] is its exit status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OwnerExit {
    /// Stopped by SIGTERM or SIGINT.
    Stopped,
    /// OpenRGB refused the connection or closed it.
    ServerUnavailable,
    /// The machine config or the drop zone cannot be used.
    Misconfigured,
}

impl OwnerExit {
    pub const fn code(self) -> i32 {
        match self {
            Self::Stopped => 0,
            Self::ServerUnavailable => 75,
            Self::Misconfigured => 78,
        }
    }
}

/// Run the owner until it stops; see the module docs. An `Err` is an
/// unexpected failure of the owner's own fds (signalfd, inotify, poll).
pub fn serve(config_path: &Path) -> io::Result<OwnerExit> {
    // Blocked before anything else, so no thread or child can take them.
    let signals = SignalFd::new(&[Signal::Terminate, Signal::Interrupt, Signal::Hangup])?;
    let mut reporter = Reporter::from_env();

    let config = match MachineConfig::load(config_path) {
        Ok(config) => config,
        Err(e) => {
            error!("{e}");
            return Ok(OwnerExit::Misconfigured);
        }
    };
    let Some(endpoint) = config.openrgb.clone() else {
        error!(
            "{} declares no OpenRGB endpoint (openrgb is null): vogix-openrgb has nothing to drive",
            config_path.display()
        );
        return Ok(OwnerExit::Misconfigured);
    };
    let zone = config.drop_zone.as_path().to_path_buf();
    let mut watch = match DropZoneWatch::new(&zone, OsStr::new(PALETTE_FILE)) {
        Ok(watch) => watch,
        Err(e) => {
            error!("cannot watch the drop zone {}: {e}", zone.display());
            return Ok(OwnerExit::Misconfigured);
        }
    };
    let mut owner = match Owner::new(config, endpoint) {
        Ok(owner) => owner,
        Err(e) => {
            error!("{}: {e}", config_path.display());
            return Ok(OwnerExit::Misconfigured);
        }
    };

    owner.load_palette();
    reporter.publish(&owner.status());
    owner.connect();

    let mut buf = vec![0u8; READ_CHUNK];
    loop {
        owner.flush();
        owner.settle();
        reporter.publish(&owner.status());
        if let Some(exit) = owner.finished() {
            return Ok(exit);
        }

        let ready = {
            let mut set = PollSet::new();
            set.add(signals.as_fd(), Interest::Read, Token::Signals);
            set.add(watch.as_fd(), Interest::Read, Token::Zone);
            if let Some(stream) = owner.stream.as_ref() {
                let interest = if owner.has_output() {
                    Interest::ReadWrite
                } else {
                    Interest::Read
                };
                set.add(stream.as_fd(), interest, Token::Socket);
            }
            set.wait()?
        };
        for (token, readiness) in ready {
            match token {
                Token::Signals => {
                    let delivered = signals.drain()?;
                    if delivered.contains(Signal::Terminate)
                        || delivered.contains(Signal::Interrupt)
                    {
                        reporter.notify(&[Notification::Stopping]);
                        owner.stop();
                    } else if delivered.contains(Signal::Hangup) {
                        owner.hangup();
                    }
                }
                Token::Zone => {
                    let events = watch.drain()?;
                    owner.on_zone(events);
                }
                Token::Socket => owner.on_socket(readiness, &mut buf),
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Token {
    Signals,
    Zone,
    Socket,
}

/// What the published palette asks of the config's OpenRGB devices.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct Wanted {
    /// Devices with a colour, keyed by device name: the session's targets.
    targets: BTreeMap<String, Target>,
    /// Devices without one, and why.
    unset: BTreeMap<DeviceName, SurfaceStatus>,
}

fn surface(state: SurfaceState, detail: String) -> SurfaceStatus {
    SurfaceStatus {
        state,
        controllers: None,
        detail: Some(detail),
    }
}

fn wire_colour(rgb: Rgb) -> RgbColor {
    let (r, g, b) = rgb.channels();
    RgbColor::from_rgb(r, g, b)
}

fn wanted(config: &MachineConfig, palette: Option<&MachinePalette>) -> Wanted {
    let mut wanted = Wanted::default();
    for (name, slot, device) in config.openrgb_devices() {
        let selector = match Selector::new(device.name_contains.as_str(), device.mode.as_str()) {
            Ok(selector) => selector,
            Err(e) => {
                wanted
                    .unset
                    .insert(name.clone(), surface(SurfaceState::Error, e.to_string()));
                continue;
            }
        };
        let Some(palette) = palette else {
            wanted.unset.insert(
                name.clone(),
                surface(SurfaceState::Waiting, "no palette is published".into()),
            );
            continue;
        };
        match palette.slot(slot) {
            Some(rgb) => {
                wanted.targets.insert(
                    name.as_str().to_owned(),
                    Target {
                        selector,
                        colour: wire_colour(rgb),
                    },
                );
            }
            None => {
                wanted.unset.insert(
                    name.clone(),
                    surface(
                        SurfaceState::Waiting,
                        format!(
                            "the published theme {} {} has no slot {slot}",
                            palette.theme.name, palette.theme.variant
                        ),
                    ),
                );
            }
        }
    }
    wanted
}

/// A device's status from the session's report on it.
fn device_surface(report: &DeviceReport, name_contains: &str) -> SurfaceStatus {
    let state = match report.state {
        DeviceState::Waiting => SurfaceState::Waiting,
        DeviceState::Absent => SurfaceState::Absent,
        DeviceState::Pending => SurfaceState::Pending,
        DeviceState::Sent => SurfaceState::Sent,
        DeviceState::Confirmed => SurfaceState::Confirmed,
        DeviceState::Error => SurfaceState::Error,
    };
    let detail = match report.state {
        DeviceState::Waiting => Some("OpenRGB has not listed its controllers yet".to_owned()),
        DeviceState::Absent => Some(format!("no controller matches {name_contains:?}")),
        DeviceState::Error => report
            .controllers
            .iter()
            .find(|controller| controller.state.is_error())
            .map(|controller| {
                format!(
                    "{} (id {}): {}",
                    controller.name, controller.dev_id, controller.state
                )
            }),
        DeviceState::Pending | DeviceState::Sent | DeviceState::Confirmed => None,
    };
    let controllers = match report.state {
        DeviceState::Waiting => None,
        _ => Some(u32::try_from(report.controllers.len()).unwrap_or(u32::MAX)),
    };
    SurfaceStatus {
        state,
        controllers,
        detail,
    }
}

fn names(available: &[String]) -> String {
    if available.is_empty() {
        "none".to_owned()
    } else {
        available.join(", ")
    }
}

/// Log one session event at the level the operator needs it.
fn log_event(event: &Event, protocol: Option<ProtocolVersion>, targets: &BTreeMap<String, Target>) {
    match event {
        Event::Mirror(event) => log_link_event(event),
        Event::Presence {
            label,
            present,
            available,
        } => {
            let text = match (present, targets.get(label)) {
                (true, _) => format!("{label}: present"),
                (false, Some(target)) => format!(
                    "{label}: no controller matches {:?} (present: {})",
                    target.selector.name_contains(),
                    names(available)
                ),
                (false, None) => format!("{label}: absent (present: {})", names(available)),
            };
            // At protocol 6 absence is reported once detection completes;
            // protocol 5 has no detection notifications, so its transitions
            // are the report.
            if protocol == Some(ProtocolVersion::V5) {
                info!("{text}");
            } else {
                debug!("{text}");
            }
        }
        Event::AbsentAtDetectionComplete {
            label,
            name_contains,
            available,
        } => warn!(
            "{label}: no controller matches {name_contains:?} (present: {})",
            names(available)
        ),
        Event::Apply {
            label,
            dev_id,
            controller,
            state,
        } => {
            let colour = targets
                .get(label)
                .map_or_else(String::new, |target| format!(" {}", target.colour));
            match state {
                ApplyState::Confirmed => {
                    info!("{label}: {controller} (id {dev_id}) confirmed{colour}")
                }
                ApplyState::Sent => info!(
                    "{label}: {controller} (index {dev_id}) sent{colour}; protocol 5 cannot confirm it"
                ),
                ApplyState::Pending => {
                    debug!("{label}: {controller} (id {dev_id}) pending{colour}")
                }
                ApplyState::Vanished => {
                    info!("{label}: {controller} (id {dev_id}): {state}")
                }
                state => error!("{label}: {controller} (id {dev_id}): {state}"),
            }
        }
        Event::StaleDiscarded { dev_id, packet } => {
            debug!("discarded {packet} for id {dev_id}, which left OpenRGB's list")
        }
    }
}

fn log_link_event(event: &MirrorEvent) {
    match event {
        MirrorEvent::Negotiated {
            server_max,
            protocol,
        } => info!("OpenRGB offers SDK protocol {server_max}; speaking protocol {protocol}"),
        MirrorEvent::ServerName(name) => info!("OpenRGB server: {name}"),
        MirrorEvent::ServerFlags(flags) => debug!("OpenRGB server flags: {flags:?}"),
        MirrorEvent::ListCommitted { controllers } => {
            debug!("OpenRGB lists {controllers} controller(s)")
        }
        MirrorEvent::ControllerRemoved { dev_id } => {
            info!("controller id {dev_id} left OpenRGB's list")
        }
        MirrorEvent::ListInconsistent => {
            debug!(
                "the controller list changed during a resync; waiting for OpenRGB's notification"
            )
        }
        MirrorEvent::DetectionStarted => info!("OpenRGB detection started"),
        MirrorEvent::DetectionProgress { percent, text } => {
            debug!("OpenRGB detection {percent}%: {text}")
        }
        MirrorEvent::DetectionComplete => info!("OpenRGB detection complete"),
        MirrorEvent::CacheRefreshed { dev_id, reason } => {
            debug!("OpenRGB updated id {dev_id} ({reason:?})")
        }
        MirrorEvent::Skipped {
            packet,
            dev_id,
            size,
        } => debug!("skipped {packet} for device {dev_id} ({size} bytes)"),
        MirrorEvent::RequestRejected {
            packet,
            dev_id,
            status,
        } => warn!("OpenRGB answered {packet} for device {dev_id} with {status}"),
        MirrorEvent::WriteAck {
            dev_id,
            packet,
            status,
        } => trace!("OpenRGB acknowledged {packet} for id {dev_id}: {status}"),
        MirrorEvent::ReadBack { dev_id, found, .. } => {
            trace!("read back id {dev_id} (found: {found})")
        }
        MirrorEvent::Faulted(fault) => error!("{fault}"),
    }
}

/// Sends `STATUS=` and writes `status.json` when the status changes.
struct Reporter {
    notifier: Option<Notifier>,
    dir: Option<PathBuf>,
    last: Option<StatusFile>,
}

impl Reporter {
    fn from_env() -> Self {
        let notifier = Notifier::from_env().unwrap_or_else(|e| {
            warn!("sd_notify is unavailable: {e}");
            None
        });
        let dir = status::runtime_directory()
            .inspect_err(|e| warn!("status.json is not written: {e}"))
            .ok();
        Self {
            notifier,
            dir,
            last: None,
        }
    }

    fn notify(&self, notifications: &[Notification]) {
        if let Some(notifier) = &self.notifier
            && let Err(e) = notifier.send(notifications)
        {
            warn!("sd_notify: {e}");
        }
    }

    fn publish(&mut self, status: &StatusFile) {
        if self.last.as_ref() == Some(status) {
            return;
        }
        let line = status.summary();
        if self.last.as_ref().map(StatusFile::summary).as_deref() != Some(line.as_str()) {
            debug!("status: {line}");
            self.notify(&[Notification::Status(StatusLine::new(line))]);
        }
        if let Some(dir) = &self.dir
            && let Err(e) = status.write(dir)
        {
            warn!("cannot write {}: {e}", dir.join(STATUS_FILE).display());
        }
        self.last = Some(status.clone());
    }
}

/// The owner's state: the config, the palette it follows, and the connection.
struct Owner {
    config: MachineConfig,
    endpoint: OpenRgbEndpoint,
    client: MirrorConfig,
    palette: Option<MachinePalette>,
    /// Why the last read of `palette.json` was not accepted.
    palette_problem: Option<String>,
    wanted: Wanted,
    /// `None` only before the first connect.
    session: Option<Session>,
    /// `None` once the connection closed.
    stream: Option<TcpStream>,
    stopping: bool,
    /// Connect anew once the faulted connection has drained (SIGHUP).
    reconnect: bool,
    /// Why the owner exits once the connection is closed.
    exit: Option<OwnerExit>,
}

impl Owner {
    fn new(config: MachineConfig, endpoint: OpenRgbEndpoint) -> Result<Self, String> {
        let client = MirrorConfig {
            max_protocol: endpoint.max_protocol,
            client_name: ClientName::new(endpoint.client_name.as_str())
                .map_err(|e| e.to_string())?,
        };
        let wanted = wanted(&config, None);
        Ok(Self {
            config,
            endpoint,
            client,
            palette: None,
            palette_problem: None,
            wanted,
            session: None,
            stream: None,
            stopping: false,
            reconnect: false,
            exit: None,
        })
    }

    fn zone(&self) -> &Path {
        self.config.drop_zone.as_path()
    }

    fn has_output(&self) -> bool {
        self.session
            .as_ref()
            .is_some_and(|session| !session.pending_output().is_empty())
    }

    /// Read `palette.json` and make its colours the targets. A file that is
    /// gone or rejected leaves the last accepted palette in force.
    fn load_palette(&mut self) {
        let path = self.zone().join(PALETTE_FILE);
        match MachinePalette::load_from_zone(self.zone()) {
            Ok(palette) => {
                if self.palette.as_ref() != Some(&palette) {
                    info!(
                        "palette: {} {} {}",
                        palette.theme.scheme, palette.theme.name, palette.theme.variant
                    );
                }
                self.palette = Some(palette);
                self.palette_problem = None;
            }
            Err(PaletteError::Absent { .. }) => {
                match &self.palette {
                    None => info!("waiting for the owner to publish {}", path.display()),
                    Some(palette) => warn!(
                        "{} is gone; keeping theme {} {}",
                        path.display(),
                        palette.theme.name,
                        palette.theme.variant
                    ),
                }
                self.palette_problem = None;
            }
            Err(e) => {
                match &self.palette {
                    None => error!("{e}"),
                    Some(palette) => error!(
                        "{e}; keeping theme {} {}",
                        palette.theme.name, palette.theme.variant
                    ),
                }
                self.palette_problem = Some(e.to_string());
            }
        }
        let wanted = wanted(&self.config, self.palette.as_ref());
        if wanted.targets != self.wanted.targets
            && let Some(session) = self.session.as_mut()
        {
            session.set_targets(wanted.targets.clone());
        }
        self.wanted = wanted;
    }

    fn connect(&mut self) {
        let (host, port) = (self.endpoint.host, self.endpoint.port);
        match connection::connect(host, port) {
            Ok(stream) => {
                info!("connected to OpenRGB at {host}:{port}");
                let mut session = Session::new(self.client.clone());
                session.set_targets(self.wanted.targets.clone());
                self.session = Some(session);
                self.stream = Some(stream);
            }
            Err(e) => {
                if e.kind() == io::ErrorKind::ConnectionRefused {
                    error!(
                        "OpenRGB at {host}:{port} refused the connection: vogix-openrgb runs only \
                         while openrgb.service is active, which it becomes once its SDK server \
                         listens (READY=1), so the server broke that readiness contract"
                    );
                } else {
                    error!("cannot connect to OpenRGB at {host}:{port}: {e}");
                }
                self.stream = None;
                self.exit = Some(OwnerExit::ServerUnavailable);
            }
        }
    }

    /// Write what the session has queued, as far as the socket takes it.
    fn flush(&mut self) {
        let (Some(stream), Some(session)) = (self.stream.as_mut(), self.session.as_mut()) else {
            return;
        };
        if session.pending_output().is_empty() {
            return;
        }
        match connection::write_available(stream, session.pending_output()) {
            Ok(written) => session.consume_output(written),
            Err(e) => self.server_closed(Some(e)),
        }
    }

    fn on_socket(&mut self, readiness: Readiness, buf: &mut [u8]) {
        if !(readiness.readable || readiness.hangup || readiness.error) {
            // Writable only: the loop's flush writes.
            return;
        }
        let (Some(stream), Some(session)) = (self.stream.as_mut(), self.session.as_mut()) else {
            return;
        };
        if let Peer::Closed { error } =
            connection::read_available(stream, buf, |bytes| session.receive(bytes))
        {
            self.server_closed(error);
        }
    }

    fn server_closed(&mut self, error: Option<io::Error>) {
        self.stream = None;
        if self.stopping {
            debug!("OpenRGB closed the connection while vogix-openrgb stops");
            return;
        }
        match error {
            None => error!("OpenRGB closed the connection"),
            Some(e) => error!("OpenRGB closed the connection: {e}"),
        }
        self.exit.get_or_insert(OwnerExit::ServerUnavailable);
    }

    /// Log what the session did, and close the connection when it may close.
    fn settle(&mut self) {
        let Some(session) = self.session.as_mut() else {
            return;
        };
        let protocol = session.protocol();
        for event in session.drain_events() {
            log_event(&event, protocol, &self.wanted.targets);
        }
        if self.stream.is_none() || !session.should_close() {
            return;
        }
        self.stream = None;
        if self.exit.is_some() {
            debug!("the OpenRGB connection drained and closed");
            return;
        }
        if self.reconnect {
            self.reconnect = false;
            info!("the faulted OpenRGB connection drained and closed; connecting anew");
            self.connect();
        } else {
            info!(
                "closed the faulted OpenRGB connection; `systemctl reload vogix-openrgb` \
                 (SIGHUP) connects anew"
            );
        }
    }

    fn stop(&mut self) {
        info!("stopping: closing the OpenRGB connection once it drains");
        self.stopping = true;
        self.exit = Some(OwnerExit::Stopped);
        if let Some(session) = self.session.as_mut() {
            session.begin_shutdown();
        }
    }

    fn hangup(&mut self) {
        if self.exit.is_some() {
            return;
        }
        info!("SIGHUP: re-reading the palette; forced re-apply of every device");
        self.load_palette();
        let faulted = self
            .session
            .as_ref()
            .is_some_and(|session| session.mirror().fault().is_some());
        if self.stream.is_none() {
            self.connect();
        } else if faulted {
            // Closed by settle() once outstanding writes are acknowledged.
            self.reconnect = true;
        } else if let Some(session) = self.session.as_mut() {
            session.force_reapply();
        }
    }

    fn on_zone(&mut self, events: ZoneEvents) {
        if events.zone_gone {
            error!(
                "the drop zone {} was removed or moved, so the owner's palette can no longer \
                 be followed",
                self.zone().display()
            );
            self.exit.get_or_insert(OwnerExit::Misconfigured);
            if let Some(session) = self.session.as_mut() {
                session.begin_shutdown();
            }
            return;
        }
        if events.overflowed {
            debug!("the drop zone's inotify queue overflowed; re-reading the palette");
        }
        if events.file_may_have_changed() {
            self.load_palette();
        }
    }

    /// The exit status, once the owner is done.
    fn finished(&self) -> Option<OwnerExit> {
        self.exit.filter(|_| self.stream.is_none())
    }

    fn status(&self) -> StatusFile {
        let link = self.session.as_ref().map(|session| session.mirror());
        let fault = link.and_then(|mirror| mirror.fault());
        let phase = if self.stopping {
            Phase::Stopping
        } else {
            match link.map(|mirror| mirror.phase()) {
                None => Phase::Starting,
                Some(LinkPhase::Faulted(_)) => Phase::Faulted,
                Some(LinkPhase::Handshaking | LinkPhase::Enumerating { .. }) => Phase::Syncing,
                Some(LinkPhase::Ready) if self.palette.is_none() => Phase::WaitingForPalette,
                Some(LinkPhase::Ready) => Phase::Ready,
            }
        };
        let mut status = StatusFile::new(OwnerUnit::Openrgb, phase);
        status.detail = fault
            .map(ToString::to_string)
            .or_else(|| self.palette_problem.clone());
        status.theme = self.palette.as_ref().map(|palette| palette.theme.clone());
        if let Some(mirror) = link {
            status.protocol = mirror.protocol().map(ProtocolVersion::number);
            status.server_name = mirror
                .server_name()
                .map(|name| name.to_string_lossy().into_owned());
            if mirror.has_committed() {
                status.controller_count =
                    Some(u32::try_from(mirror.controllers().count()).unwrap_or(u32::MAX));
            }
        }
        let reports: BTreeMap<String, DeviceReport> = self
            .session
            .as_ref()
            .map(|session| {
                session
                    .device_reports()
                    .into_iter()
                    .map(|report| (report.label.clone(), report))
                    .collect()
            })
            .unwrap_or_default();
        for (name, _, device) in self.config.openrgb_devices() {
            let surface = match (self.wanted.unset.get(name), reports.get(name.as_str())) {
                (Some(unset), _) => unset.clone(),
                (None, Some(report)) => device_surface(report, device.name_contains.as_str()),
                (None, None) => {
                    surface(SurfaceState::Waiting, "not connected to OpenRGB yet".into())
                }
            };
            status.devices.insert(name.clone(), surface);
        }
        status
    }
}

#[cfg(test)]
mod tests {
    use super::super::session::ControllerReport;
    use super::*;

    fn config() -> MachineConfig {
        MachineConfig::from_json(
            br#"{
                "schema": 1, "owner": "t", "dropZone": "/var/lib/vogix/machine",
                "console": { "enable": true },
                "openrgb": { "host": "127.0.0.1", "port": 6742, "maxProtocol": 6, "clientName": "vogix" },
                "devices": {
                    "dram-rgb": { "slot": "base01",
                        "provider": { "openrgb": { "nameContains": "ENE DRAM", "mode": "Static" } } },
                    "govee": { "slot": "base0D",
                        "provider": { "openrgb": { "nameContains": "Govee", "mode": "Static" } } },
                    "strip": { "slot": "accent_missing",
                        "provider": { "openrgb": { "nameContains": "Strip", "mode": "Direct" } } },
                    "kraken-ring": { "slot": "base01",
                        "provider": { "command": { "argv": ["/bin/liquidctl", "{{color}}"] } } }
                }
            }"#,
        )
        .unwrap()
    }

    fn palette() -> MachinePalette {
        serde_json::from_value(serde_json::json!({
            "schema": 1,
            "theme": { "scheme": "vogix16", "name": "nordic", "variant": "dark" },
            "slots": { "base01": "#3b4252", "base0D": "#81A1C1" },
            "console": null
        }))
        .unwrap()
    }

    fn name(s: &str) -> DeviceName {
        s.parse().unwrap()
    }

    #[test]
    fn exit_codes_are_the_sysexits_the_units_expect() {
        assert_eq!(OwnerExit::Stopped.code(), 0);
        assert_eq!(OwnerExit::ServerUnavailable.code(), 75);
        assert_eq!(OwnerExit::Misconfigured.code(), 78);
    }

    #[test]
    fn a_palette_gives_every_openrgb_device_with_its_slot_a_target() {
        let wanted = wanted(&config(), Some(&palette()));
        assert_eq!(
            wanted
                .targets
                .keys()
                .map(String::as_str)
                .collect::<Vec<_>>(),
            ["dram-rgb", "govee"],
            "command devices are the local owner's"
        );
        let dram = &wanted.targets["dram-rgb"];
        assert_eq!(dram.selector.name_contains(), "ENE DRAM");
        assert_eq!(dram.selector.mode(), "Static");
        assert_eq!(dram.colour, RgbColor::from_rgb(0x3b, 0x42, 0x52));
        assert_eq!(
            wanted.targets["govee"].colour,
            RgbColor::from_rgb(0x81, 0xa1, 0xc1)
        );

        let strip = &wanted.unset[&name("strip")];
        assert_eq!(strip.state, SurfaceState::Waiting);
        assert_eq!(
            strip.detail.as_deref(),
            Some("the published theme nordic dark has no slot accent_missing")
        );
    }

    #[test]
    fn without_a_palette_every_openrgb_device_waits() {
        let wanted = wanted(&config(), None);
        assert!(wanted.targets.is_empty());
        assert_eq!(
            wanted
                .unset
                .keys()
                .map(DeviceName::as_str)
                .collect::<Vec<_>>(),
            ["dram-rgb", "govee", "strip"]
        );
        assert!(
            wanted
                .unset
                .values()
                .all(|s| s.state == SurfaceState::Waiting
                    && s.detail.as_deref() == Some("no palette is published"))
        );
    }

    fn report(state: DeviceState, controllers: Vec<ControllerReport>) -> DeviceReport {
        DeviceReport {
            label: "dram-rgb".into(),
            state,
            controllers,
        }
    }

    fn controller(dev_id: u32, state: ApplyState) -> ControllerReport {
        ControllerReport {
            dev_id,
            name: "ENE DRAM".into(),
            state,
        }
    }

    #[test]
    fn device_reports_become_status_surfaces() {
        let confirmed = device_surface(
            &report(
                DeviceState::Confirmed,
                vec![
                    controller(1, ApplyState::Confirmed),
                    controller(2, ApplyState::Confirmed),
                ],
            ),
            "ENE DRAM",
        );
        assert_eq!(
            confirmed,
            SurfaceStatus {
                state: SurfaceState::Confirmed,
                controllers: Some(2),
                detail: None,
            }
        );

        let absent = device_surface(&report(DeviceState::Absent, vec![]), "Keychron K2 HE");
        assert_eq!(absent.state, SurfaceState::Absent);
        assert_eq!(absent.controllers, Some(0));
        assert_eq!(
            absent.detail.as_deref(),
            Some("no controller matches \"Keychron K2 HE\"")
        );

        let rejected = device_surface(
            &report(
                DeviceState::Error,
                vec![
                    controller(1, ApplyState::Confirmed),
                    controller(
                        2,
                        ApplyState::Rejected {
                            packet: super::super::wire::PacketId::UpdateLeds,
                            status: super::super::model::AckStatus::ErrorInvalidData,
                        },
                    ),
                ],
            ),
            "ENE DRAM",
        );
        assert_eq!(rejected.state, SurfaceState::Error);
        let detail = rejected.detail.unwrap();
        assert!(
            detail.starts_with("ENE DRAM (id 2): OpenRGB answered"),
            "{detail}"
        );

        let waiting = device_surface(&report(DeviceState::Waiting, vec![]), "ENE DRAM");
        assert_eq!(waiting.state, SurfaceState::Waiting);
        assert_eq!(waiting.controllers, None);
    }

    #[test]
    fn before_connecting_the_status_names_every_device() {
        let config = config();
        let endpoint = config.openrgb.clone().unwrap();
        let owner = Owner::new(config, endpoint).unwrap();
        let status = owner.status();
        assert_eq!(status.phase, Phase::Starting);
        assert_eq!(status.owner, OwnerUnit::Openrgb);
        assert_eq!(status.protocol, None);
        assert_eq!(
            status
                .devices
                .keys()
                .map(DeviceName::as_str)
                .collect::<Vec<_>>(),
            ["dram-rgb", "govee", "strip"]
        );
        assert!(
            status
                .devices
                .values()
                .all(|s| s.state == SurfaceState::Waiting)
        );
        assert!(status.problems().is_empty());
    }

    #[test]
    fn a_rejected_palette_leaves_the_last_accepted_one_in_force() {
        let zone = tempfile::tempdir().unwrap();
        let mut config = config();
        config.drop_zone = zone.path().to_path_buf().try_into().unwrap();
        let endpoint = config.openrgb.clone().unwrap();
        let mut owner = Owner::new(config, endpoint).unwrap();
        let file = zone.path().join(PALETTE_FILE);

        owner.load_palette();
        assert!(owner.palette.is_none() && owner.palette_problem.is_none());
        assert!(owner.wanted.targets.is_empty());

        std::fs::write(&file, palette().to_json()).unwrap();
        owner.load_palette();
        assert_eq!(owner.wanted.targets.len(), 2);
        let accepted = owner.wanted.clone();

        std::fs::write(&file, b"{").unwrap();
        owner.load_palette();
        assert_eq!(
            owner.wanted, accepted,
            "the rejected file changes no target"
        );
        let problem = owner.palette_problem.clone().unwrap();
        assert!(problem.contains(&file.display().to_string()), "{problem}");
        let status = owner.status();
        assert_eq!(status.detail.as_deref(), Some(problem.as_str()));
        assert_eq!(status.theme.unwrap().name.as_str(), "nordic");

        std::fs::remove_file(&file).unwrap();
        owner.load_palette();
        assert_eq!(owner.wanted, accepted);
        assert_eq!(owner.palette_problem, None);
    }

    #[test]
    fn a_session_with_targets_reports_them_waiting_until_the_list_is_committed() {
        let config = config();
        let endpoint = config.openrgb.clone().unwrap();
        let mut owner = Owner::new(config, endpoint).unwrap();
        owner.palette = Some(palette());
        owner.wanted = wanted(&owner.config, owner.palette.as_ref());
        let mut session = Session::new(owner.client.clone());
        session.set_targets(owner.wanted.targets.clone());
        owner.session = Some(session);

        let status = owner.status();
        assert_eq!(status.phase, Phase::Syncing);
        assert_eq!(status.theme.as_ref().unwrap().name.as_str(), "nordic");
        let dram = &status.devices[&name("dram-rgb")];
        assert_eq!(dram.state, SurfaceState::Waiting);
        assert_eq!(
            dram.detail.as_deref(),
            Some("OpenRGB has not listed its controllers yet")
        );
        assert_eq!(
            status.devices[&name("strip")].detail.as_deref(),
            Some("the published theme nordic dark has no slot accent_missing")
        );
    }

    // The tests below drive the owner's own connection handling over real
    // loopback sockets whose far end is a test-held listener writing scripted
    // bytes. They are not evidence of protocol compatibility with OpenRGB; the
    // openrgb-owner VM check is.

    /// The handshake the owner sends first: REQUEST_PROTOCOL_VERSION (16-byte
    /// header and a u32) then a REQUEST_CONTROLLER_COUNT header.
    const HANDSHAKE_BYTES: usize = 20 + 16;

    struct Loopback {
        listener: std::net::TcpListener,
        zone: tempfile::TempDir,
        buf: Vec<u8>,
    }

    impl Loopback {
        fn new() -> Self {
            Self {
                listener: std::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0)).unwrap(),
                zone: tempfile::tempdir().unwrap(),
                buf: vec![0; READ_CHUNK],
            }
        }

        fn owner(&self) -> Owner {
            let mut config = config();
            config.drop_zone = self.zone.path().to_path_buf().try_into().unwrap();
            let endpoint = config.openrgb.as_mut().unwrap();
            endpoint.port = self.listener.local_addr().unwrap().port();
            let endpoint = endpoint.clone();
            Owner::new(config, endpoint).unwrap()
        }

        /// Accept the owner's connection and read its handshake.
        fn accept(&self, owner: &mut Owner) -> TcpStream {
            use std::io::Read;
            let (mut peer, _) = self.listener.accept().unwrap();
            owner.flush();
            let mut hello = [0u8; HANDSHAKE_BYTES];
            peer.read_exact(&mut hello).unwrap();
            assert_eq!(&hello[..4], b"ORGB");
            assert_eq!(&hello[20..24], b"ORGB");
            peer
        }

        /// Wait until the owner's socket is readable, then let it read.
        fn deliver(&mut self, owner: &mut Owner) {
            let readiness = {
                let stream = owner.stream.as_ref().expect("connected");
                let mut set = PollSet::new();
                set.add(stream.as_fd(), Interest::Read, ());
                set.wait().unwrap()[0].1
            };
            owner.on_socket(readiness, &mut self.buf);
            owner.settle();
        }
    }

    #[test]
    fn a_protocol_fault_closes_the_connection_and_sighup_connects_anew() {
        use std::io::{Read, Write};
        let mut lo = Loopback::new();
        let mut owner = lo.owner();
        owner.connect();
        let mut peer = lo.accept(&mut owner);

        // A header without the ORGB magic cannot be framed: the owner closes
        // at once, reports the fault and keeps running.
        peer.write_all(b"XRGB\0\0\0\0\0\0\0\0\0\0\0\0").unwrap();
        lo.deliver(&mut owner);
        assert!(owner.stream.is_none());
        assert_eq!(owner.finished(), None, "a fault does not exit");
        let status = owner.status();
        assert_eq!(status.phase, Phase::Faulted);
        let detail = status.detail.unwrap();
        assert!(detail.contains("cannot be framed"), "{detail}");
        let mut rest = Vec::new();
        peer.read_to_end(&mut rest).unwrap();
        assert!(rest.is_empty(), "the owner sent nothing after the fault");

        // SIGHUP connects anew with a fresh session.
        owner.hangup();
        let second = lo.accept(&mut owner);
        assert!(owner.stream.is_some());
        assert_eq!(owner.status().phase, Phase::Syncing);

        // The server closing that connection ends the owner with 75.
        drop(second);
        lo.deliver(&mut owner);
        assert_eq!(owner.finished(), Some(OwnerExit::ServerUnavailable));
    }

    #[test]
    fn a_stop_closes_once_the_output_is_flushed_and_exits_zero() {
        use std::io::Read;
        let lo = Loopback::new();
        let mut owner = lo.owner();
        owner.connect();
        let (mut peer, _) = lo.listener.accept().unwrap();
        owner.stop();
        assert_eq!(owner.finished(), None, "the handshake is not written yet");
        owner.flush();
        owner.settle();
        assert_eq!(owner.finished(), Some(OwnerExit::Stopped));
        assert_eq!(owner.status().phase, Phase::Stopping);
        let mut sent = Vec::new();
        peer.read_to_end(&mut sent).unwrap();
        assert_eq!(sent.len(), HANDSHAKE_BYTES, "flushed, then closed");
    }

    #[test]
    fn a_removed_drop_zone_ends_the_owner_with_78_after_the_drain() {
        let lo = Loopback::new();
        let mut owner = lo.owner();
        owner.connect();
        let _peer = lo.listener.accept().unwrap();
        owner.on_zone(ZoneEvents {
            zone_gone: true,
            ..ZoneEvents::default()
        });
        assert_eq!(owner.finished(), None, "the handshake is not written yet");
        owner.flush();
        owner.settle();
        assert_eq!(owner.finished(), Some(OwnerExit::Misconfigured));
    }

    #[test]
    fn a_refused_connect_ends_the_owner_with_75() {
        let lo = Loopback::new();
        let mut owner = lo.owner();
        drop(lo.listener);
        owner.connect();
        assert!(owner.stream.is_none());
        assert_eq!(owner.finished(), Some(OwnerExit::ServerUnavailable));
    }
}

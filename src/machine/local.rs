//! `vogix machine serve local`: the local machine owner
//! (`vogix-machine.service`, root).
//!
//! It follows the palette the machine owner publishes into the drop zone
//! and applies it to the surfaces that need no server: the kernel's VT
//! palette and the command devices. Every wake-up is an event — a palette
//! renamed into the drop zone, a hidraw node of a declared USB device
//! appearing, a child exiting, SIGHUP or a stop — waited for in one
//! `poll(2)` with no timeout.
//!
//! Startup reads the palette, reconciles the VT palette and only then sends
//! `READY=1`, whatever the reconcile's outcome, so units ordered after this
//! one (the login gettys and greeters) draw with the owner's colours. The
//! command devices run after that.

use super::command::{CommandSet, Finished, Requested, RunOutcome};
use super::config::{HidrawMatch, MachineConfig};
use super::console::{self, ColourMap, Reconciled};
use super::exit::OwnerExit;
use super::notify::{Notification, Notifier, StatusLine};
use super::palette::{MachinePalette, PALETTE_FILE, PaletteError, ThemeRef};
use super::reactor::{Delivered, DropZoneWatch, Interest, PollSet, Signal, SignalFd, ZoneEvents};
use super::status::{self, OwnerUnit, Phase, StatusFile, SurfaceState, SurfaceStatus};
use super::types::{DeviceName, Rgb, SlotName};
use super::uevent::{Arrivals, HidrawMonitor};
use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::os::fd::AsFd;
use std::path::{Path, PathBuf};

/// Where the kernel exposes sysfs; hidraw nodes are under `class/hidraw`.
const SYSFS: &str = "/sys";

/// Run the local owner until it is stopped. `config_path` is the machine
/// config (`/etc/vogix/machine.json`).
pub fn serve(config_path: &Path) -> OwnerExit {
    // First, before anything could start a thread: the handled signals are
    // blocked and read from a signalfd.
    let signals = match SignalFd::new(&[
        Signal::Terminate,
        Signal::Interrupt,
        Signal::Hangup,
        Signal::Child,
    ]) {
        Ok(signals) => signals,
        Err(e) => {
            log::error!("cannot set up the signalfd: {e}");
            return OwnerExit::TempFail;
        }
    };
    let notifier = match Notifier::from_env() {
        Ok(notifier) => notifier,
        Err(e) => {
            log::error!("no service-manager notifications: {e}");
            None
        }
    };
    let status_dir = match status::runtime_directory() {
        Ok(dir) => dir,
        Err(e) => {
            log::error!("{e}");
            return OwnerExit::Config;
        }
    };
    let config = match MachineConfig::load(config_path) {
        Ok(config) => config,
        Err(e) => {
            log::error!("{e}");
            return OwnerExit::Config;
        }
    };

    let zone_path = config.drop_zone.as_path().to_path_buf();
    let mut zone = match DropZoneWatch::new(&zone_path, OsStr::new(PALETTE_FILE)) {
        Ok(zone) => zone,
        Err(e) => {
            log::error!("cannot watch the drop zone {}: {e}", zone_path.display());
            return OwnerExit::drop_zone_unwatchable(&e);
        }
    };
    let hotplug_declared = config
        .command_devices()
        .any(|(_, _, device)| device.hotplug.hidraw.is_some());
    let mut hidraw = if hotplug_declared {
        match HidrawMonitor::open(Path::new(SYSFS)) {
            Ok((monitor, present)) => {
                for node in &present {
                    log::debug!("hidraw present: {} ({})", node.node, node.hid);
                }
                Some(monitor)
            }
            Err(e) => {
                log::error!("cannot watch hidraw uevents: {e}");
                return OwnerExit::TempFail;
            }
        }
    } else {
        None
    };

    let mut owner = LocalOwner::new(config, PathBuf::from(console::CONSOLE_DEVICE));
    owner.accept(MachinePalette::load_from_zone(&zone_path));
    owner.reconcile_console();
    let mut reporter = Reporter::new(status_dir, notifier);
    reporter.report(&owner.status());
    reporter.notify(&[Notification::Ready]);
    owner.reconcile_devices(Force::None);
    reporter.report(&owner.status());

    loop {
        let ready = {
            let mut set = PollSet::new();
            set.add(signals.as_fd(), Interest::Read, Source::Signals);
            set.add(zone.as_fd(), Interest::Read, Source::Zone);
            if let Some(monitor) = &hidraw {
                set.add(monitor.as_fd(), Interest::Read, Source::Hidraw);
            }
            set.wait()
        };
        let ready = match ready {
            Ok(ready) => ready,
            Err(e) => {
                log::error!("poll: {e}");
                return OwnerExit::TempFail;
            }
        };

        let mut delivered = Delivered::default();
        let mut zone_events = ZoneEvents::default();
        let mut arrivals = Arrivals::default();
        for (source, _) in ready {
            let drained = match source {
                Source::Signals => signals.drain().map(|d| delivered = d),
                Source::Zone => zone.drain().map(|z| zone_events = z),
                Source::Hidraw => match hidraw.as_mut() {
                    Some(monitor) => monitor.drain().map(|a| arrivals = a),
                    None => Ok(()),
                },
            };
            if let Err(e) = drained {
                log::error!("reading {source:?} events: {e}");
                return OwnerExit::TempFail;
            }
        }

        if delivered.contains(Signal::Terminate) || delivered.contains(Signal::Interrupt) {
            log::info!("stopping");
            reporter.notify(&[Notification::Stopping]);
            return OwnerExit::Stopped;
        }
        if delivered.contains(Signal::Child) {
            owner.reap();
        }
        if zone_events.zone_gone {
            log::error!(
                "the drop zone {} was removed or moved; the watch on it is gone",
                zone_path.display()
            );
            return OwnerExit::drop_zone_gone();
        }
        let forced = delivered.contains(Signal::Hangup);
        if forced {
            log::info!("forced re-apply (SIGHUP)");
        }
        if forced || zone_events.file_may_have_changed() {
            owner.accept(MachinePalette::load_from_zone(&zone_path));
            owner.reconcile_console();
            owner.reconcile_devices(if forced { Force::All } else { Force::None });
        }
        if !arrivals.devices.is_empty() {
            owner.on_arrivals(&arrivals);
        }
        reporter.report(&owner.status());
    }
}

/// A pollable event source of the owner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Source {
    Signals,
    Zone,
    Hidraw,
}

/// Which command devices run even when they already hold their colour.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Force {
    None,
    All,
    Device(DeviceName),
}

/// One command device as the owner tracks it.
#[derive(Debug)]
struct CommandTrack {
    slot: SlotName,
    hidraw: Option<HidrawMatch>,
    /// The colour of the last run that exited 0.
    holds: Option<Rgb>,
    /// The run in flight: its pid and colour.
    running: Option<(u32, Rgb)>,
    /// The colour of the run in flight, or of the run queued behind it.
    pending: Option<Rgb>,
    /// Why the last run (or its spawn) failed; cleared by the next request.
    failure: Option<String>,
}

/// The local owner's state: the palette it follows and where each surface
/// stands.
#[derive(Debug)]
struct LocalOwner {
    console_enabled: bool,
    console_device: PathBuf,
    /// The last palette accepted from the drop zone.
    palette: Option<MachinePalette>,
    /// Why the drop zone's current palette was rejected, while it is.
    rejected: Option<String>,
    console: SurfaceStatus,
    commands: CommandSet,
    tracks: BTreeMap<DeviceName, CommandTrack>,
}

impl LocalOwner {
    fn new(config: MachineConfig, console_device: PathBuf) -> Self {
        let tracks = config
            .command_devices()
            .map(|(name, slot, device)| {
                (
                    name.clone(),
                    CommandTrack {
                        slot: slot.clone(),
                        hidraw: device.hotplug.hidraw,
                        holds: None,
                        running: None,
                        pending: None,
                        failure: None,
                    },
                )
            })
            .collect();
        Self {
            console_enabled: config.console.enable,
            console_device,
            palette: None,
            rejected: None,
            console: waiting("no palette has been published"),
            commands: CommandSet::from_config(&config),
            tracks,
        }
    }

    /// Take a (re-)read of the drop zone. An absent palette keeps what the
    /// owner follows; a rejected one keeps the last accepted palette and is
    /// reported until a valid one is published.
    fn accept(&mut self, loaded: Result<MachinePalette, PaletteError>) {
        match loaded {
            Ok(palette) => {
                if self.palette.as_ref() != Some(&palette) {
                    let t = &palette.theme;
                    log::info!("palette: {} {} {}", t.scheme, t.name, t.variant);
                }
                self.rejected = None;
                self.palette = Some(palette);
            }
            Err(PaletteError::Absent { path }) => {
                log::info!("no palette published yet ({})", path.display());
            }
            Err(e) => {
                log::error!("palette rejected: {e}");
                self.rejected = Some(e.to_string());
            }
        }
    }

    /// Compare the kernel's VT palette with the published console colours
    /// and write them when they differ.
    fn reconcile_console(&mut self) {
        if !self.console_enabled {
            return;
        }
        let Some(palette) = &self.palette else {
            self.console = waiting("no palette has been published");
            return;
        };
        let Some(colours) = &palette.console else {
            self.console = waiting("the published palette has no console colours");
            return;
        };
        let theme = describe(&palette.theme);
        self.console =
            match console::reconcile(&self.console_device, &ColourMap::from_colours(colours)) {
                Ok(Reconciled::Written) => {
                    log::info!("console: applied the {theme} palette");
                    SurfaceStatus::new(SurfaceState::Confirmed)
                }
                Ok(Reconciled::AlreadyCurrent) => {
                    log::info!("console: the {theme} palette is already current");
                    SurfaceStatus::new(SurfaceState::Confirmed)
                }
                Err(e) => {
                    log::error!("console: {e}");
                    SurfaceStatus {
                        state: SurfaceState::Error,
                        controllers: None,
                        detail: Some(e.to_string()),
                    }
                }
            };
    }

    /// Run every command device that does not end on its slot's colour (the
    /// colour of its run in flight or queued, else the colour it holds), plus
    /// those `force` names.
    fn reconcile_devices(&mut self, force: Force) {
        let names: Vec<DeviceName> = self.tracks.keys().cloned().collect();
        for name in names {
            let forced = match &force {
                Force::None => false,
                Force::All => true,
                Force::Device(device) => *device == name,
            };
            self.reconcile_device(&name, forced);
        }
    }

    fn reconcile_device(&mut self, name: &DeviceName, forced: bool) {
        let Some(track) = self.tracks.get_mut(name) else {
            return;
        };
        let Some(colour) = self.palette.as_ref().and_then(|p| p.slot(&track.slot)) else {
            return;
        };
        // The colour the device ends on: the run in flight or queued, else
        // the last run that succeeded.
        if !forced && track.pending.or(track.holds) == Some(colour) {
            return;
        }
        track.failure = None;
        match self.commands.request(name, colour) {
            Some(Requested::Started { pid }) => {
                track.running = Some((pid, colour));
                track.pending = Some(colour);
            }
            Some(Requested::Queued) => track.pending = Some(colour),
            Some(Requested::SpawnFailed(e)) => {
                track.pending = None;
                track.failure = Some(format!("cannot start the command: {e}"));
            }
            None => {}
        }
    }

    /// After SIGCHLD: reap every exited run and record how it ended.
    fn reap(&mut self) {
        for (name, finished) in self.commands.reap_all() {
            if let Some(track) = self.tracks.get_mut(&name) {
                track.record(finished);
            }
        }
    }

    /// hidraw nodes appeared: re-run every command device whose declared
    /// USB ids match one of them.
    fn on_arrivals(&mut self, arrivals: &Arrivals) {
        let matched: Vec<DeviceName> = self
            .tracks
            .iter()
            .filter_map(|(name, track)| {
                let wanted = track.hidraw?;
                let node = arrivals.devices.iter().find(|d| d.hid.matches(&wanted))?;
                log::info!(
                    "{name}: hidraw {} ({wanted}) {}; re-running",
                    node.node,
                    if arrivals.rescanned {
                        "present after a uevent overflow"
                    } else {
                        "appeared"
                    }
                );
                Some(name.clone())
            })
            .collect();
        for name in matched {
            self.reconcile_devices(Force::Device(name));
        }
    }

    /// The status file this state reads as.
    fn status(&self) -> StatusFile {
        let phase = match (&self.palette, &self.rejected) {
            (Some(_), None) => Phase::Ready,
            _ => Phase::WaitingForPalette,
        };
        let mut status = StatusFile::new(OwnerUnit::Local, phase);
        status.detail = self
            .rejected
            .as_ref()
            .map(|why| format!("the published palette is rejected: {why}"));
        status.theme = self.palette.as_ref().map(|p| p.theme.clone());
        if self.console_enabled {
            status.console = Some(self.console.clone());
        }
        for (name, track) in &self.tracks {
            status
                .devices
                .insert(name.clone(), track.surface(self.palette.as_ref()));
        }
        status
    }
}

impl CommandTrack {
    /// A run ended; when the device was dirty, the runner started the
    /// queued colour, which `pending` holds.
    fn record(&mut self, finished: Finished) {
        self.running = None;
        if matches!(finished.outcome, RunOutcome::Succeeded) {
            self.holds = Some(finished.color);
        }
        self.failure = match &finished.outcome {
            RunOutcome::Succeeded => None,
            RunOutcome::Failed(status) => Some(format!(
                "the command (pid {}) failed: {status}",
                finished.pid
            )),
            RunOutcome::WaitFailed(e) => Some(format!(
                "cannot wait for the command (pid {}): {e}",
                finished.pid
            )),
        };
        match finished.rerun {
            Some(Requested::Started { pid }) => {
                self.running = self.pending.map(|colour| (pid, colour));
            }
            Some(Requested::Queued) | None => self.pending = None,
            Some(Requested::SpawnFailed(e)) => {
                self.pending = None;
                self.failure = Some(format!("cannot start the command: {e}"));
            }
        }
    }

    fn surface(&self, palette: Option<&MachinePalette>) -> SurfaceStatus {
        let Some(palette) = palette else {
            return waiting("no palette has been published");
        };
        let Some(target) = palette.slot(&self.slot) else {
            return waiting(&format!(
                "slot {} is not in the published palette",
                self.slot
            ));
        };
        if let Some(next) = self.pending {
            let detail = match self.running {
                Some((pid, colour)) if colour == next => format!("pid {pid} applying {colour}"),
                Some((pid, colour)) => format!("pid {pid} applying {colour}; {next} next"),
                None => format!("{next} next"),
            };
            return SurfaceStatus {
                state: SurfaceState::Pending,
                controllers: None,
                detail: Some(detail),
            };
        }
        if let Some(failure) = &self.failure {
            return SurfaceStatus {
                state: SurfaceState::Error,
                controllers: None,
                detail: Some(failure.clone()),
            };
        }
        if self.holds == Some(target) {
            SurfaceStatus::new(SurfaceState::Confirmed)
        } else {
            SurfaceStatus::new(SurfaceState::Waiting)
        }
    }
}

fn waiting(why: &str) -> SurfaceStatus {
    SurfaceStatus {
        state: SurfaceState::Waiting,
        controllers: None,
        detail: Some(why.to_string()),
    }
}

fn describe(theme: &ThemeRef) -> String {
    format!("{} {} {}", theme.scheme, theme.name, theme.variant)
}

/// Writes `status.json` and sends `STATUS=` whenever the status changes.
struct Reporter {
    dir: PathBuf,
    notifier: Option<Notifier>,
    last: Option<StatusFile>,
}

impl Reporter {
    fn new(dir: PathBuf, notifier: Option<Notifier>) -> Self {
        Self {
            dir,
            notifier,
            last: None,
        }
    }

    fn report(&mut self, status: &StatusFile) {
        if self.last.as_ref() == Some(status) {
            return;
        }
        if let Err(e) = status.write(&self.dir) {
            log::error!(
                "cannot write {}: {e}",
                self.dir.join(status::STATUS_FILE).display()
            );
        }
        self.notify(&[Notification::Status(StatusLine::new(status.summary()))]);
        self.last = Some(status.clone());
    }

    fn notify(&self, notifications: &[Notification]) {
        if let Some(notifier) = &self.notifier
            && let Err(e) = notifier.send(notifications)
        {
            log::error!("cannot notify the service manager: {e}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::machine::types::Rgb;
    use crate::machine::uevent::{HidId, Hidraw};
    use std::fs;

    /// Block until `pid` has exited, leaving it for `try_wait` to reap
    /// (waitid with WNOWAIT) — the event SIGCHLD stands for.
    fn wait_exited(pid: u32) {
        // SAFETY: an all-zero siginfo_t is a valid out-parameter.
        let mut info: libc::siginfo_t = unsafe { std::mem::zeroed() };
        // SAFETY: `info` is valid for writes; P_PID with our own child's pid.
        let rc = unsafe {
            libc::waitid(
                libc::P_PID,
                pid as libc::id_t,
                &mut info,
                libc::WEXITED | libc::WNOWAIT,
            )
        };
        assert_eq!(rc, 0, "waitid: {}", std::io::Error::last_os_error());
    }

    /// A config with one command device that appends its colour to `log`
    /// and exits with the status in `exit_file` (0 when absent).
    fn config(dir: &Path, console: bool) -> MachineConfig {
        let script = dir.join("device.sh");
        fs::write(
            &script,
            format!(
                "#!/bin/sh\nprintf '%s\\n' \"$1\" >> {log}\nexit $(cat {exit} 2>/dev/null || echo 0)\n",
                log = dir.join("log").display(),
                exit = dir.join("exit").display()
            ),
        )
        .unwrap();
        let json = serde_json::json!({
            "schema": 1, "owner": "t", "dropZone": dir,
            "console": { "enable": console }, "openrgb": null,
            "devices": {
                "ring": { "slot": "base01", "provider": { "command": {
                    "argv": ["/bin/sh", script, "{{color}}"],
                    "hotplug": { "hidraw": { "vendorId": "1e71", "productId": "3012" } }
                } } }
            }
        });
        MachineConfig::from_json(json.to_string().as_bytes()).unwrap()
    }

    fn palette(base01: Rgb, console: bool) -> MachinePalette {
        let json = serde_json::json!({
            "schema": 1,
            "theme": { "scheme": "vogix16", "name": "nordic", "variant": "dark" },
            "slots": { "base01": base01.to_string() },
            "console": if console {
                serde_json::json!((0..16).map(|i| format!("#{i:02x}0000")).collect::<Vec<_>>())
            } else {
                serde_json::Value::Null
            }
        });
        serde_json::from_value(json).unwrap()
    }

    fn ring() -> DeviceName {
        "ring".parse().unwrap()
    }

    fn device_state(owner: &LocalOwner) -> SurfaceStatus {
        owner.status().devices[&ring()].clone()
    }

    fn log_lines(dir: &Path) -> Vec<String> {
        fs::read_to_string(dir.join("log"))
            .unwrap_or_default()
            .lines()
            .map(String::from)
            .collect()
    }

    /// Wait for the device's run to exit and reap it, as SIGCHLD would.
    fn finish_run(owner: &mut LocalOwner) {
        let (pid, _) = owner.tracks[&ring()].running.unwrap();
        wait_exited(pid);
        owner.reap();
    }

    const NORD1: Rgb = Rgb::new(0x3b, 0x42, 0x52);
    const DESERT1: Rgb = Rgb::new(0x33, 0x33, 0x33);

    #[test]
    fn with_no_palette_everything_waits_and_nothing_runs() {
        let dir = tempfile::tempdir().unwrap();
        let mut owner = LocalOwner::new(config(dir.path(), true), dir.path().join("tty0"));
        owner.accept(MachinePalette::load_from_zone(dir.path()));
        owner.reconcile_console();
        owner.reconcile_devices(Force::None);
        let status = owner.status();
        assert_eq!(status.phase, Phase::WaitingForPalette);
        assert_eq!(status.console.unwrap().state, SurfaceState::Waiting);
        assert_eq!(device_state(&owner).state, SurfaceState::Waiting);
        assert!(owner.tracks[&ring()].running.is_none());
    }

    #[test]
    fn a_device_runs_with_its_slot_colour_and_is_confirmed_on_exit_0() {
        let dir = tempfile::tempdir().unwrap();
        let mut owner = LocalOwner::new(config(dir.path(), false), dir.path().join("tty0"));
        owner.accept(Ok(palette(NORD1, false)));
        owner.reconcile_devices(Force::None);
        assert_eq!(device_state(&owner).state, SurfaceState::Pending);
        finish_run(&mut owner);
        assert_eq!(log_lines(dir.path()), ["3b4252"]);
        let status = owner.status();
        assert_eq!(status.phase, Phase::Ready);
        assert_eq!(status.devices[&ring()].state, SurfaceState::Confirmed);
        assert!(status.console.is_none(), "console.enable is off");

        // The same colour again is no run; a forced apply is one.
        owner.accept(Ok(palette(NORD1, false)));
        owner.reconcile_devices(Force::None);
        assert!(owner.tracks[&ring()].running.is_none());
        owner.reconcile_devices(Force::All);
        finish_run(&mut owner);
        assert_eq!(log_lines(dir.path()), ["3b4252", "3b4252"]);
    }

    #[test]
    fn a_failed_run_is_an_error_until_a_later_run_succeeds() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("exit"), "3").unwrap();
        let mut owner = LocalOwner::new(config(dir.path(), false), dir.path().join("tty0"));
        owner.accept(Ok(palette(NORD1, false)));
        owner.reconcile_devices(Force::None);
        finish_run(&mut owner);
        let state = device_state(&owner);
        assert_eq!(state.state, SurfaceState::Error);
        assert!(
            state.detail.as_deref().unwrap().contains("exit status: 3"),
            "{state:?}"
        );
        assert_eq!(
            owner.status().problems(),
            [format!("ring: error: {}", state.detail.unwrap())]
        );

        // The device does not hold its colour, so the next palette event
        // runs it again even with the same colour.
        fs::remove_file(dir.path().join("exit")).unwrap();
        owner.accept(Ok(palette(NORD1, false)));
        owner.reconcile_devices(Force::None);
        finish_run(&mut owner);
        assert_eq!(device_state(&owner).state, SurfaceState::Confirmed);
    }

    #[test]
    fn changes_during_a_run_cost_one_rerun_with_the_last_colour() {
        let dir = tempfile::tempdir().unwrap();
        let mut owner = LocalOwner::new(config(dir.path(), false), dir.path().join("tty0"));
        owner.accept(Ok(palette(NORD1, false)));
        owner.reconcile_devices(Force::None);
        let (first, _) = owner.tracks[&ring()].running.unwrap();
        for colour in [DESERT1, Rgb::new(1, 2, 3), DESERT1] {
            owner.accept(Ok(palette(colour, false)));
            owner.reconcile_devices(Force::None);
        }
        wait_exited(first);
        owner.reap();
        assert_eq!(device_state(&owner).state, SurfaceState::Pending);
        finish_run(&mut owner);
        assert_eq!(log_lines(dir.path()), ["3b4252", "333333"]);
        assert_eq!(device_state(&owner).state, SurfaceState::Confirmed);
    }

    #[test]
    fn a_palette_back_at_the_held_colour_during_a_run_reruns_with_it() {
        let dir = tempfile::tempdir().unwrap();
        let mut owner = LocalOwner::new(config(dir.path(), false), dir.path().join("tty0"));
        owner.accept(Ok(palette(NORD1, false)));
        owner.reconcile_devices(Force::None);
        finish_run(&mut owner);
        assert_eq!(device_state(&owner).state, SurfaceState::Confirmed);

        // The device holds NORD1; a run for DESERT1 starts, and the palette
        // is back at NORD1 before that run is reaped.
        owner.accept(Ok(palette(DESERT1, false)));
        owner.reconcile_devices(Force::None);
        let (running, _) = owner.tracks[&ring()].running.unwrap();
        owner.accept(Ok(palette(NORD1, false)));
        owner.reconcile_devices(Force::None);
        assert_eq!(
            device_state(&owner).detail.as_deref(),
            Some(format!("pid {running} applying #333333; #3b4252 next").as_str())
        );
        wait_exited(running);
        owner.reap();
        finish_run(&mut owner);
        assert_eq!(log_lines(dir.path()), ["3b4252", "333333", "3b4252"]);
        assert_eq!(device_state(&owner).state, SurfaceState::Confirmed);
    }

    #[test]
    fn a_matching_hidraw_arrival_reruns_its_device_and_others_do_not() {
        let dir = tempfile::tempdir().unwrap();
        let mut owner = LocalOwner::new(config(dir.path(), false), dir.path().join("tty0"));
        owner.accept(Ok(palette(NORD1, false)));
        owner.reconcile_devices(Force::None);
        finish_run(&mut owner);

        let node = |vendor, product| Hidraw {
            node: "hidraw3".into(),
            hid: HidId {
                bus: 3,
                vendor,
                product,
            },
        };
        owner.on_arrivals(&Arrivals {
            devices: vec![node(0x0627, 0x0001)],
            rescanned: false,
        });
        assert!(owner.tracks[&ring()].running.is_none());

        owner.on_arrivals(&Arrivals {
            devices: vec![node(0x0627, 0x0001), node(0x1e71, 0x3012)],
            rescanned: false,
        });
        finish_run(&mut owner);
        assert_eq!(log_lines(dir.path()), ["3b4252", "3b4252"]);
    }

    #[test]
    fn a_missing_slot_waits_and_names_the_slot() {
        let dir = tempfile::tempdir().unwrap();
        let mut owner = LocalOwner::new(config(dir.path(), false), dir.path().join("tty0"));
        let mut p = palette(NORD1, false);
        p.slots.clear();
        owner.accept(Ok(p));
        owner.reconcile_devices(Force::All);
        let state = device_state(&owner);
        assert_eq!(state.state, SurfaceState::Waiting);
        assert_eq!(
            state.detail.as_deref(),
            Some("slot base01 is not in the published palette")
        );
        assert!(owner.tracks[&ring()].running.is_none());
    }

    #[test]
    fn a_rejected_palette_keeps_the_last_one_and_waits_for_a_valid_one() {
        let dir = tempfile::tempdir().unwrap();
        let mut owner = LocalOwner::new(config(dir.path(), false), dir.path().join("tty0"));
        owner.accept(Ok(palette(NORD1, false)));
        owner.reconcile_devices(Force::None);
        finish_run(&mut owner);

        fs::write(dir.path().join(PALETTE_FILE), b"{\"schema\": 1}").unwrap();
        owner.accept(MachinePalette::load_from_zone(dir.path()));
        owner.reconcile_devices(Force::None);
        let status = owner.status();
        assert_eq!(status.phase, Phase::WaitingForPalette);
        assert!(
            status
                .detail
                .as_deref()
                .unwrap()
                .starts_with("the published palette is rejected: "),
            "{status:?}"
        );
        assert_eq!(status.theme.unwrap().name.as_str(), "nordic");
        assert_eq!(status.devices[&ring()].state, SurfaceState::Confirmed);

        owner.accept(Ok(palette(DESERT1, false)));
        assert_eq!(owner.status().phase, Phase::Ready);
    }

    #[test]
    fn a_console_write_failure_is_an_error_and_a_palette_without_console_waits() {
        let dir = tempfile::tempdir().unwrap();
        // /dev/null is no VT: the owner reports the GIO_CMAP failure.
        let mut owner = LocalOwner::new(config(dir.path(), true), PathBuf::from("/dev/null"));
        owner.accept(Ok(palette(NORD1, true)));
        owner.reconcile_console();
        let console = owner.status().console.unwrap();
        assert_eq!(console.state, SurfaceState::Error);
        assert!(
            console
                .detail
                .as_deref()
                .unwrap()
                .starts_with("GIO_CMAP on /dev/null"),
            "{console:?}"
        );

        owner.accept(Ok(palette(NORD1, false)));
        owner.reconcile_console();
        let console = owner.status().console.unwrap();
        assert_eq!(console.state, SurfaceState::Waiting);
        assert_eq!(
            console.detail.as_deref(),
            Some("the published palette has no console colours")
        );
    }

    #[test]
    fn the_reporter_writes_only_changes() {
        let dir = tempfile::tempdir().unwrap();
        let mut reporter = Reporter::new(dir.path().to_path_buf(), None);
        let status = StatusFile::new(OwnerUnit::Local, Phase::Ready);
        reporter.report(&status);
        let path = dir.path().join(status::STATUS_FILE);
        let inode = |p: &Path| std::os::unix::fs::MetadataExt::ino(&fs::metadata(p).unwrap());
        let first = inode(&path);
        reporter.report(&status);
        assert_eq!(inode(&path), first);
        assert_eq!(StatusFile::read(&path).unwrap(), status);
    }
}

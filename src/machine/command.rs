//! Command devices: a declared argv run with the device's colour, one run at
//! a time per device.
//!
//! A request while the device's command is still running only marks the
//! device dirty, remembering the newest colour; when the run exits (the owner
//! sees SIGCHLD and reaps), one more run starts with that colour. Any number
//! of requests during a run therefore cost at most one further run, and the
//! last colour always wins.
//!
//! Children get an empty signal mask, no stdin and the owner's stdout and
//! stderr (the unit's journal), and do not see `$NOTIFY_SOCKET`.

use super::config::{Argv, MachineConfig};
use super::notify::NOTIFY_SOCKET_ENV;
use super::reactor::UnblockSignals;
use super::types::{DeviceName, Rgb};
use std::collections::BTreeMap;
use std::io;
use std::process::{Child, Command, ExitStatus, Stdio};

/// What a request did.
#[derive(Debug)]
pub enum Requested {
    /// The command was spawned.
    Started { pid: u32 },
    /// A run is in progress; the colour runs next, after it exits.
    Queued,
    /// The command could not be spawned.
    SpawnFailed(io::Error),
}

/// How a finished run ended.
#[derive(Debug)]
pub enum RunOutcome {
    /// Exited with status 0.
    Succeeded,
    /// Exited non-zero or was killed by a signal.
    Failed(ExitStatus),
    /// Waiting for the child failed.
    WaitFailed(io::Error),
}

/// A run that ended, and the rerun it started when the device was dirty.
#[derive(Debug)]
pub struct Finished {
    pub pid: u32,
    pub color: Rgb,
    pub outcome: RunOutcome,
    pub rerun: Option<Requested>,
}

#[derive(Debug)]
struct Running {
    child: Child,
    color: Rgb,
}

/// One command device's runner.
#[derive(Debug)]
pub struct SingleFlight {
    device: DeviceName,
    argv: Argv,
    running: Option<Running>,
    /// The dirty flag: the newest colour requested during the current run.
    queued: Option<Rgb>,
}

impl SingleFlight {
    pub fn new(device: DeviceName, argv: Argv) -> Self {
        Self {
            device,
            argv,
            running: None,
            queued: None,
        }
    }

    #[cfg(test)]
    pub fn running_pid(&self) -> Option<u32> {
        self.running.as_ref().map(|r| r.child.id())
    }

    /// Run the command with `color`, or queue it behind the current run.
    pub fn request(&mut self, color: Rgb) -> Requested {
        if self.running.is_some() {
            self.queued = Some(color);
            log::debug!("{}: {color} queued behind the current run", self.device);
            return Requested::Queued;
        }
        self.spawn(color)
    }

    fn spawn(&mut self, color: Rgb) -> Requested {
        let argv = self.argv.render(color);
        let spawned = Command::new(&argv[0])
            .args(&argv[1..])
            .stdin(Stdio::null())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .env_remove(NOTIFY_SOCKET_ENV)
            .unblock_signals()
            .spawn();
        match spawned {
            Ok(child) => {
                let pid = child.id();
                log::info!(
                    "{}: started {} for {color} (pid {pid})",
                    self.device,
                    self.argv.program()
                );
                self.running = Some(Running { child, color });
                Requested::Started { pid }
            }
            Err(e) => {
                log::error!("{}: cannot start {}: {e}", self.device, self.argv.program());
                Requested::SpawnFailed(e)
            }
        }
    }

    /// Reap the current run if it has exited, and start the queued colour if
    /// the device is dirty. `None` while the run is still going or when
    /// nothing runs.
    pub fn reap(&mut self) -> Option<Finished> {
        let running = self.running.as_mut()?;
        let pid = running.child.id();
        let outcome = match running.child.try_wait() {
            Ok(None) => return None,
            Ok(Some(status)) if status.success() => RunOutcome::Succeeded,
            Ok(Some(status)) => RunOutcome::Failed(status),
            Err(e) => RunOutcome::WaitFailed(e),
        };
        let color = running.color;
        self.running = None;
        match &outcome {
            RunOutcome::Succeeded => log::info!("{}: pid {pid} exited 0", self.device),
            RunOutcome::Failed(status) => {
                log::warn!("{}: pid {pid} failed: {status}", self.device)
            }
            RunOutcome::WaitFailed(e) => {
                log::error!("{}: cannot wait for pid {pid}: {e}", self.device)
            }
        }
        let rerun = self.queued.take().map(|next| self.spawn(next));
        Some(Finished {
            pid,
            color,
            outcome,
            rerun,
        })
    }
}

/// Every command device of a machine config, by name.
#[derive(Debug, Default)]
pub struct CommandSet {
    devices: BTreeMap<DeviceName, SingleFlight>,
}

impl CommandSet {
    pub fn from_config(config: &MachineConfig) -> Self {
        Self {
            devices: config
                .command_devices()
                .map(|(name, _, device)| {
                    (
                        name.clone(),
                        SingleFlight::new(name.clone(), device.argv.clone()),
                    )
                })
                .collect(),
        }
    }

    /// Request a run of `device`; `None` when no such command device exists.
    pub fn request(&mut self, device: &DeviceName, color: Rgb) -> Option<Requested> {
        self.devices.get_mut(device).map(|d| d.request(color))
    }

    /// On SIGCHLD: reap every device whose run exited (SIGCHLD coalesces, so
    /// every running child is checked), starting queued reruns.
    pub fn reap_all(&mut self) -> Vec<(DeviceName, Finished)> {
        self.devices
            .iter_mut()
            .filter_map(|(name, device)| device.reap().map(|f| (name.clone(), f)))
            .collect()
    }

    #[cfg(test)]
    pub fn get(&self, device: &DeviceName) -> Option<&SingleFlight> {
        self.devices.get(device)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::machine::reactor::{Signal, SignalFd};
    use std::fs::{self, OpenOptions};
    use std::io::Write;
    use std::path::Path;

    fn argv(parts: &[&str]) -> Argv {
        Argv::try_from(parts.iter().map(|s| s.to_string()).collect::<Vec<_>>()).unwrap()
    }

    fn device() -> DeviceName {
        "kraken-ring".parse().unwrap()
    }

    /// Block until `pid` has exited, leaving it for `try_wait` to reap
    /// (waitid with WNOWAIT) — the event the owner's SIGCHLD stands for.
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
        assert_eq!(rc, 0, "waitid: {}", io::Error::last_os_error());
    }

    fn started(requested: &Requested) -> u32 {
        match requested {
            Requested::Started { pid } => *pid,
            other => panic!("expected a start, got {other:?}"),
        }
    }

    /// Let one blocked run through its FIFO gate.
    fn release(fifo: &Path) {
        // Opening for writing blocks until the run opens it for reading.
        let mut gate = OpenOptions::new().write(true).open(fifo).unwrap();
        gate.write_all(b"go\n").unwrap();
    }

    #[test]
    fn exit_statuses_are_reported() {
        let mut ok = SingleFlight::new(device(), argv(&["/bin/sh", "-c", "exit 0"]));
        let pid = started(&ok.request(Rgb::new(1, 2, 3)));
        wait_exited(pid);
        let finished = ok.reap().unwrap();
        assert_eq!(finished.pid, pid);
        assert!(matches!(finished.outcome, RunOutcome::Succeeded));
        assert!(finished.rerun.is_none());
        assert!(ok.reap().is_none(), "nothing runs any more");

        let mut failing = SingleFlight::new(device(), argv(&["/bin/sh", "-c", "exit 3"]));
        let pid = started(&failing.request(Rgb::new(1, 2, 3)));
        wait_exited(pid);
        match failing.reap().unwrap().outcome {
            RunOutcome::Failed(status) => assert_eq!(status.code(), Some(3)),
            other => panic!("expected exit 3, got {other:?}"),
        }
    }

    #[test]
    fn a_missing_program_is_a_spawn_failure_not_a_stuck_run() {
        let mut absent = SingleFlight::new(device(), argv(&["/nonexistent/vogix-test-program"]));
        match absent.request(Rgb::new(0, 0, 0)) {
            Requested::SpawnFailed(e) => assert_eq!(e.kind(), io::ErrorKind::NotFound),
            other => panic!("expected a spawn failure, got {other:?}"),
        }
        assert_eq!(absent.running_pid(), None);
        assert!(matches!(
            absent.request(Rgb::new(0, 0, 0)),
            Requested::SpawnFailed(_)
        ));
    }

    #[test]
    fn requests_during_a_run_cost_exactly_one_rerun_with_the_last_colour() {
        let dir = tempfile::tempdir().unwrap();
        let fifo = dir.path().join("gate");
        let log = dir.path().join("runs");
        let fifo_c = std::ffi::CString::new(fifo.to_str().unwrap()).unwrap();
        // SAFETY: a valid NUL-terminated path.
        assert_eq!(unsafe { libc::mkfifo(fifo_c.as_ptr(), 0o600) }, 0);

        // Each run waits at the FIFO gate, then appends its colour argument.
        let mut runner = SingleFlight::new(
            device(),
            argv(&[
                "/bin/sh",
                "-c",
                "read -r _ < \"$2\"; printf '%s\\n' \"$1\" >> \"$3\"",
                "sh",
                "{{color}}",
                fifo.to_str().unwrap(),
                log.to_str().unwrap(),
            ]),
        );

        let first = started(&runner.request(Rgb::new(0x11, 0x11, 0x11)));
        for c in [0x22, 0x33, 0x44] {
            assert!(matches!(
                runner.request(Rgb::new(c, c, c)),
                Requested::Queued
            ));
        }
        assert!(
            runner.reap().is_none(),
            "the first run is still at its gate"
        );

        release(&fifo);
        wait_exited(first);
        let finished = runner.reap().unwrap();
        assert_eq!(finished.pid, first);
        assert_eq!(finished.color, Rgb::new(0x11, 0x11, 0x11));
        let second = started(finished.rerun.as_ref().unwrap());
        assert_ne!(second, first);

        release(&fifo);
        wait_exited(second);
        let finished = runner.reap().unwrap();
        assert_eq!(finished.color, Rgb::new(0x44, 0x44, 0x44));
        assert!(finished.rerun.is_none(), "exactly one rerun");
        assert_eq!(runner.running_pid(), None);

        assert_eq!(fs::read_to_string(&log).unwrap(), "111111\n444444\n");
    }

    #[test]
    fn a_run_started_under_the_owners_signal_mask_has_none_blocked() {
        let _signals = SignalFd::new(&[
            Signal::Terminate,
            Signal::Interrupt,
            Signal::Hangup,
            Signal::Child,
        ])
        .unwrap();
        // The shell reports its own mask; bash unblocks SIGCHLD for itself,
        // so this shows the SIGHUP, SIGINT and SIGTERM bits (reactor's test
        // reads all four through `cat`).
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("sigblk");
        let mut runner = SingleFlight::new(
            device(),
            argv(&[
                "/bin/sh",
                "-c",
                "while read -r key value; do \
                   if [ \"$key\" = SigBlk: ]; then printf '%s' \"$value\" > \"$1\"; fi; \
                 done < /proc/self/status",
                "sh",
                out.to_str().unwrap(),
            ]),
        );
        let pid = started(&runner.request(Rgb::new(0, 0, 0)));
        wait_exited(pid);
        assert!(matches!(
            runner.reap().unwrap().outcome,
            RunOutcome::Succeeded
        ));
        assert_eq!(fs::read_to_string(&out).unwrap(), "0000000000000000");
    }

    #[test]
    fn the_set_holds_every_command_device_and_only_those() {
        let config = MachineConfig::from_json(
            br#"{
                "schema": 1, "owner": "t", "dropZone": "/var/lib/vogix/machine",
                "console": { "enable": false },
                "openrgb": { "host": "127.0.0.1", "port": 6742, "maxProtocol": 6, "clientName": "vogix" },
                "devices": {
                    "dram-rgb": { "slot": "base01",
                        "provider": { "openrgb": { "nameContains": "ENE DRAM", "mode": "Static" } } },
                    "kraken-ring": { "slot": "base01",
                        "provider": { "command": { "argv": ["/bin/sh", "-c", "exit 0"] } } }
                }
            }"#,
        )
        .unwrap();
        let mut set = CommandSet::from_config(&config);
        assert!(set.get(&"dram-rgb".parse().unwrap()).is_none());
        assert!(
            set.request(&"dram-rgb".parse().unwrap(), Rgb::new(0, 0, 0))
                .is_none()
        );
        let pid = started(&set.request(&device(), Rgb::new(0, 0, 0)).unwrap());
        wait_exited(pid);
        let reaped = set.reap_all();
        assert_eq!(reaped.len(), 1);
        assert_eq!(reaped[0].0, device());
        assert!(set.reap_all().is_empty());
    }
}

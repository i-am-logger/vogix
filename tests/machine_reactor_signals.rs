//! A machine owner's signals, end to end, in a process shaped like an owner.
//!
//! Signal masks are per thread, and SIGCHLD and a `kill` are directed at the
//! process: under the libtest harness a thread that does not block them may
//! take them before the signalfd does. This test therefore runs without the
//! harness (`harness = false` in Cargo.toml), single-threaded as the owners
//! are, and includes `src/machine/reactor.rs` itself because vogix is a
//! binary crate.
//!
//! It checks that a child's exit wakes the poll through the signalfd as
//! SIGCHLD, that the child ran with nothing blocked, and that a
//! process-directed SIGTERM and SIGHUP arrive the same way.

#[allow(dead_code)]
#[path = "../src/machine/reactor.rs"]
mod reactor;

use reactor::{Interest, PollSet, Signal, SignalFd, UnblockSignals};
use std::os::fd::AsFd;
use std::process::{Command, Stdio};

fn wait_for(signals: &SignalFd) -> reactor::Delivered {
    let mut set = PollSet::new();
    set.add(signals.as_fd(), Interest::Read, ());
    let ready = set.wait().expect("poll");
    assert_eq!(ready.len(), 1);
    assert!(ready[0].1.readable);
    signals.drain().expect("drain the signalfd")
}

fn main() {
    let signals = SignalFd::new(&[
        Signal::Terminate,
        Signal::Interrupt,
        Signal::Hangup,
        Signal::Child,
    ])
    .expect("block the owner's signals and open a signalfd");

    // The child reports its own blocked mask, then exits 7.
    let child = Command::new("/bin/sh")
        .args([
            "-c",
            "while read -r key value; do \
               if [ \"$key\" = SigBlk: ]; then printf '%s' \"$value\"; fi; \
             done < /proc/self/status; exit 7",
        ])
        .stdout(Stdio::piped())
        .unblock_signals()
        .spawn()
        .expect("spawn sh");

    let delivered = wait_for(&signals);
    assert!(delivered.contains(Signal::Child), "{delivered:?}");
    assert!(!delivered.contains(Signal::Terminate));

    let output = child.wait_with_output().expect("reap the child");
    assert_eq!(output.status.code(), Some(7));
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "0000000000000000",
        "the child started with nothing blocked"
    );

    // SAFETY: kill to this process with signals it blocks.
    assert_eq!(unsafe { libc::kill(libc::getpid(), libc::SIGTERM) }, 0);
    let delivered = wait_for(&signals);
    assert!(delivered.contains(Signal::Terminate), "{delivered:?}");

    // SAFETY: as above.
    assert_eq!(unsafe { libc::kill(libc::getpid(), libc::SIGHUP) }, 0);
    let delivered = wait_for(&signals);
    assert!(delivered.contains(Signal::Hangup), "{delivered:?}");
    assert!(signals.drain().expect("drain").is_empty());

    println!("machine_reactor_signals: SIGCHLD, SIGTERM and SIGHUP arrived through the signalfd");
}

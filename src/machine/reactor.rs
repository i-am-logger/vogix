//! The machine owners' event sources and their one wait.
//!
//! An owner blocks the signals it handles and reads them from a signalfd,
//! watches the drop zone with inotify, and waits for any of its fds in
//! `poll(2)` with no timeout. Nothing here sleeps, retries or times out:
//! every wake-up is an event.
//!
//! This file depends only on std, libc, inotify and log.

use std::ffi::{OsStr, OsString};
use std::io;
use std::marker::PhantomData;
use std::mem::MaybeUninit;
use std::os::fd::{AsFd, AsRawFd, BorrowedFd, FromRawFd, OwnedFd};
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::Command;

/// A signal the machine owners handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Signal {
    /// SIGTERM: stop.
    Terminate,
    /// SIGINT: stop.
    Interrupt,
    /// SIGHUP: force a re-apply.
    Hangup,
    /// SIGCHLD: a child exited.
    Child,
}

impl Signal {
    pub const fn number(self) -> libc::c_int {
        match self {
            Self::Terminate => libc::SIGTERM,
            Self::Interrupt => libc::SIGINT,
            Self::Hangup => libc::SIGHUP,
            Self::Child => libc::SIGCHLD,
        }
    }

    fn from_number(number: u32) -> Option<Self> {
        [Self::Terminate, Self::Interrupt, Self::Hangup, Self::Child]
            .into_iter()
            .find(|s| s.number() as u32 == number)
    }

    const fn bit(self) -> u8 {
        match self {
            Self::Terminate => 1,
            Self::Interrupt => 2,
            Self::Hangup => 4,
            Self::Child => 8,
        }
    }
}

/// The signals delivered since the last drain. Standard signals coalesce in
/// the kernel, so this is a set, not a count.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Delivered(u8);

impl Delivered {
    pub fn contains(self, signal: Signal) -> bool {
        self.0 & signal.bit() != 0
    }

    pub fn is_empty(self) -> bool {
        self.0 == 0
    }

    fn insert(&mut self, signal: Signal) {
        self.0 |= signal.bit();
    }
}

fn sigset_of(signals: &[Signal]) -> io::Result<libc::sigset_t> {
    let mut set = MaybeUninit::<libc::sigset_t>::uninit();
    // SAFETY: sigemptyset initializes the set it is given.
    if unsafe { libc::sigemptyset(set.as_mut_ptr()) } != 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: initialized by sigemptyset above.
    let mut set = unsafe { set.assume_init() };
    for signal in signals {
        // SAFETY: `set` is an initialized sigset_t and the number a valid signal.
        if unsafe { libc::sigaddset(&mut set, signal.number()) } != 0 {
            return Err(io::Error::last_os_error());
        }
    }
    Ok(set)
}

/// Signals read from a file descriptor instead of interrupting the process.
#[derive(Debug)]
pub struct SignalFd {
    fd: OwnedFd,
}

impl SignalFd {
    /// Block `signals` for the calling thread, then open a non-blocking
    /// signalfd for them.
    ///
    /// An owner calls this first, before it spawns any thread: a thread
    /// inherits its creator's mask, and a thread that does not block a
    /// process-directed signal would take it instead of the signalfd.
    /// Children must not inherit the blocked mask; spawn them through
    /// [`UnblockSignals::unblock_signals`].
    pub fn new(signals: &[Signal]) -> io::Result<Self> {
        let set = sigset_of(signals)?;
        // SAFETY: `set` is an initialized sigset_t; a null old-set pointer is allowed.
        let rc = unsafe { libc::pthread_sigmask(libc::SIG_BLOCK, &set, std::ptr::null_mut()) };
        if rc != 0 {
            return Err(io::Error::from_raw_os_error(rc));
        }
        // SAFETY: -1 asks for a new fd; `set` is initialized.
        let fd = unsafe { libc::signalfd(-1, &set, libc::SFD_CLOEXEC | libc::SFD_NONBLOCK) };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: signalfd returned a fresh fd that nothing else owns.
        Ok(Self {
            fd: unsafe { OwnedFd::from_raw_fd(fd) },
        })
    }

    /// Read every pending signal.
    pub fn drain(&self) -> io::Result<Delivered> {
        const RECORD: usize = std::mem::size_of::<libc::signalfd_siginfo>();
        let mut delivered = Delivered::default();
        let mut records = [MaybeUninit::<libc::signalfd_siginfo>::uninit(); 8];
        loop {
            // SAFETY: the buffer is valid for writes of its full size.
            let n = unsafe {
                libc::read(
                    self.fd.as_raw_fd(),
                    records.as_mut_ptr().cast(),
                    RECORD * records.len(),
                )
            };
            if n < 0 {
                let err = io::Error::last_os_error();
                match err.kind() {
                    io::ErrorKind::WouldBlock => return Ok(delivered),
                    io::ErrorKind::Interrupted => continue,
                    _ => return Err(err),
                }
            }
            let n = n as usize;
            if n == 0 || !n.is_multiple_of(RECORD) {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("signalfd read {n} bytes, not whole {RECORD}-byte records"),
                ));
            }
            for record in &records[..n / RECORD] {
                // SAFETY: the kernel wrote this whole record.
                let signo = unsafe { record.assume_init_ref() }.ssi_signo;
                match Signal::from_number(signo) {
                    Some(signal) => delivered.insert(signal),
                    None => log::debug!("signalfd delivered unhandled signal {signo}"),
                }
            }
        }
    }
}

impl AsFd for SignalFd {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.fd.as_fd()
    }
}

/// Spawning children with an empty signal mask.
pub trait UnblockSignals {
    /// Reset the child's signal mask to empty between fork and exec. Rust's
    /// `Command` leaves the parent's mask in place, and an owner blocks
    /// SIGTERM, SIGINT, SIGHUP and SIGCHLD for its signalfd; a child that
    /// kept them blocked would ignore systemd's SIGTERM on stop.
    fn unblock_signals(&mut self) -> &mut Self;
}

impl UnblockSignals for Command {
    fn unblock_signals(&mut self) -> &mut Self {
        // SAFETY: the closure runs in the forked child before exec and calls
        // only sigemptyset and pthread_sigmask, both async-signal-safe; it
        // allocates nothing and touches no lock.
        unsafe {
            self.pre_exec(|| {
                let mut set = MaybeUninit::<libc::sigset_t>::uninit();
                if libc::sigemptyset(set.as_mut_ptr()) != 0 {
                    return Err(io::Error::last_os_error());
                }
                let rc =
                    libc::pthread_sigmask(libc::SIG_SETMASK, set.as_ptr(), std::ptr::null_mut());
                if rc != 0 {
                    return Err(io::Error::from_raw_os_error(rc));
                }
                Ok(())
            })
        }
    }
}

/// What a registered fd is waited for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Interest {
    Read,
    ReadWrite,
}

/// What `poll` reported for one fd.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Readiness {
    pub readable: bool,
    pub writable: bool,
    /// The peer closed (POLLHUP).
    pub hangup: bool,
    /// An error is pending on the fd (POLLERR).
    pub error: bool,
}

/// One `poll(2)` over borrowed fds, each tagged with a caller token. Built
/// per wait from the owner's live fds; the borrow keeps every fd open for
/// as long as the set exists.
#[derive(Debug)]
pub struct PollSet<'fd, T> {
    fds: Vec<libc::pollfd>,
    tokens: Vec<T>,
    _borrow: PhantomData<BorrowedFd<'fd>>,
}

impl<'fd, T: Copy> PollSet<'fd, T> {
    pub fn new() -> Self {
        Self {
            fds: Vec::new(),
            tokens: Vec::new(),
            _borrow: PhantomData,
        }
    }

    pub fn add(&mut self, fd: BorrowedFd<'fd>, interest: Interest, token: T) {
        let events = match interest {
            Interest::Read => libc::POLLIN,
            Interest::ReadWrite => libc::POLLIN | libc::POLLOUT,
        };
        self.fds.push(libc::pollfd {
            fd: fd.as_raw_fd(),
            events,
            revents: 0,
        });
        self.tokens.push(token);
    }

    /// Block until at least one fd is ready — no timeout — and return the
    /// ready ones in registration order. A closed fd in the set (POLLNVAL)
    /// is an error, never a busy loop.
    pub fn wait(&mut self) -> io::Result<Vec<(T, Readiness)>> {
        if self.fds.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "poll over no fds would never return",
            ));
        }
        loop {
            // SAFETY: `fds` is a valid array of `len` pollfd structs.
            let n =
                unsafe { libc::poll(self.fds.as_mut_ptr(), self.fds.len() as libc::nfds_t, -1) };
            if n >= 0 {
                break;
            }
            let err = io::Error::last_os_error();
            // A signal handler ran (none of the owners' own signals: those
            // are blocked); the wait simply continues.
            if err.kind() != io::ErrorKind::Interrupted {
                return Err(err);
            }
        }
        let mut ready = Vec::new();
        for (pfd, token) in self.fds.iter().zip(&self.tokens) {
            if pfd.revents & libc::POLLNVAL != 0 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("fd {} in the poll set is not open", pfd.fd),
                ));
            }
            if pfd.revents != 0 {
                ready.push((
                    *token,
                    Readiness {
                        readable: pfd.revents & libc::POLLIN != 0,
                        writable: pfd.revents & libc::POLLOUT != 0,
                        hangup: pfd.revents & libc::POLLHUP != 0,
                        error: pfd.revents & libc::POLLERR != 0,
                    },
                ));
            }
        }
        Ok(ready)
    }
}

impl<T: Copy> Default for PollSet<'_, T> {
    fn default() -> Self {
        Self::new()
    }
}

/// What happened in the drop zone since the last drain.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ZoneEvents {
    /// A new file was renamed onto the watched name.
    pub replaced: bool,
    /// The kernel's inotify queue overflowed; events were lost, so the file
    /// may have been replaced.
    pub overflowed: bool,
    /// The drop zone directory was removed or moved; the watch is gone.
    pub zone_gone: bool,
}

impl ZoneEvents {
    /// Whether the watched file must be read again.
    pub fn file_may_have_changed(self) -> bool {
        self.replaced || self.overflowed
    }
}

/// An inotify watch on the drop zone directory for one file name. Publishers
/// replace the file by renaming a temp file onto it, so the event is
/// IN_MOVED_TO with that name; temp files and other names are ignored.
#[derive(Debug)]
pub struct DropZoneWatch {
    inotify: inotify::Inotify,
    file: OsString,
    buf: Vec<u8>,
}

impl DropZoneWatch {
    pub fn new(zone: &Path, file: &OsStr) -> io::Result<Self> {
        use inotify::WatchMask;
        let inotify = inotify::Inotify::init()?;
        inotify.watches().add(
            zone,
            WatchMask::MOVED_TO
                | WatchMask::DELETE_SELF
                | WatchMask::MOVE_SELF
                | WatchMask::ONLYDIR,
        )?;
        Ok(Self {
            inotify,
            file: file.to_os_string(),
            buf: vec![0; 4096],
        })
    }

    /// Read every queued event and fold them into one [`ZoneEvents`], so any
    /// number of replacements since the last wait cost one re-read.
    pub fn drain(&mut self) -> io::Result<ZoneEvents> {
        use inotify::EventMask;
        let mut seen = ZoneEvents::default();
        loop {
            let events = match self.inotify.read_events(&mut self.buf) {
                Ok(events) => events,
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => return Ok(seen),
                Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(e),
            };
            for event in events {
                if event.mask.contains(EventMask::Q_OVERFLOW) {
                    seen.overflowed = true;
                }
                if event
                    .mask
                    .intersects(EventMask::DELETE_SELF | EventMask::MOVE_SELF | EventMask::IGNORED)
                {
                    seen.zone_gone = true;
                }
                if event.mask.contains(EventMask::MOVED_TO)
                    && event.name == Some(self.file.as_os_str())
                {
                    seen.replaced = true;
                }
            }
        }
    }
}

impl AsFd for DropZoneWatch {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.inotify.as_fd()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// Deliver `signal` to this test's own thread, which blocks it.
    fn raise_on_this_thread(signal: Signal) {
        // SAFETY: pthread_self is always valid for the calling thread.
        let rc = unsafe { libc::pthread_kill(libc::pthread_self(), signal.number()) };
        assert_eq!(rc, 0);
    }

    fn replace(dir: &Path, name: &str, bytes: &[u8]) {
        let tmp = dir.join(format!(".{name}.tmp"));
        fs::write(&tmp, bytes).unwrap();
        fs::rename(&tmp, dir.join(name)).unwrap();
    }

    #[test]
    fn a_blocked_signal_is_read_from_the_signalfd_and_coalesces() {
        let signals = SignalFd::new(&[Signal::Hangup, Signal::Terminate, Signal::Child]).unwrap();
        assert!(signals.drain().unwrap().is_empty());

        raise_on_this_thread(Signal::Hangup);
        raise_on_this_thread(Signal::Hangup);
        raise_on_this_thread(Signal::Child);
        let mut set = PollSet::new();
        set.add(signals.as_fd(), Interest::Read, "signals");
        let ready = set.wait().unwrap();
        assert_eq!(ready.len(), 1);
        assert!(ready[0].1.readable);

        let delivered = signals.drain().unwrap();
        assert!(delivered.contains(Signal::Hangup));
        assert!(delivered.contains(Signal::Child));
        assert!(!delivered.contains(Signal::Terminate));
        assert!(signals.drain().unwrap().is_empty(), "drained to empty");
    }

    #[test]
    fn a_replaced_palette_is_one_event_however_often_it_was_replaced() {
        let zone = tempfile::tempdir().unwrap();
        let mut watch = DropZoneWatch::new(zone.path(), OsStr::new("palette.json")).unwrap();
        assert_eq!(watch.drain().unwrap(), ZoneEvents::default());

        for i in 0..3u8 {
            replace(zone.path(), "palette.json", &[i]);
        }
        let mut set = PollSet::new();
        set.add(watch.as_fd(), Interest::Read, ());
        assert_eq!(set.wait().unwrap().len(), 1);
        drop(set);

        let seen = watch.drain().unwrap();
        assert!(seen.replaced && seen.file_may_have_changed());
        assert!(!seen.zone_gone && !seen.overflowed);
        assert_eq!(watch.drain().unwrap(), ZoneEvents::default());
    }

    #[test]
    fn other_names_and_in_place_writes_are_not_replacements() {
        let zone = tempfile::tempdir().unwrap();
        let mut watch = DropZoneWatch::new(zone.path(), OsStr::new("palette.json")).unwrap();
        replace(zone.path(), "other.json", b"x");
        // The temp file's own creation and writes are not watched events.
        fs::write(zone.path().join(".palette.json.tmp.1"), b"x").unwrap();
        assert_eq!(watch.drain().unwrap(), ZoneEvents::default());
    }

    #[test]
    fn a_removed_zone_is_reported() {
        let parent = tempfile::tempdir().unwrap();
        let zone = parent.path().join("machine");
        fs::create_dir(&zone).unwrap();
        let mut watch = DropZoneWatch::new(&zone, OsStr::new("palette.json")).unwrap();
        fs::remove_dir(&zone).unwrap();
        let seen = watch.drain().unwrap();
        assert!(seen.zone_gone);
        assert!(!seen.replaced);
    }

    #[test]
    fn the_zone_must_be_a_directory() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("f");
        fs::write(&file, b"").unwrap();
        assert!(DropZoneWatch::new(&file, OsStr::new("palette.json")).is_err());
    }

    #[test]
    fn poll_reports_writability_and_refuses_an_empty_set() {
        let (a, _b) = std::os::unix::net::UnixStream::pair().unwrap();
        let mut set = PollSet::new();
        set.add(a.as_fd(), Interest::ReadWrite, 7u8);
        let ready = set.wait().unwrap();
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].0, 7);
        assert!(ready[0].1.writable && !ready[0].1.readable);

        assert!(PollSet::<()>::new().wait().is_err());
    }

    #[test]
    fn poll_reports_a_closed_peer_as_hangup() {
        let (a, b) = std::os::unix::net::UnixStream::pair().unwrap();
        drop(b);
        let mut set = PollSet::new();
        set.add(a.as_fd(), Interest::Read, ());
        let ready = set.wait().unwrap();
        assert!(ready[0].1.hangup);
    }

    /// `cat` from PATH: a probe that, unlike a shell, leaves its inherited
    /// signal mask untouched (bash unblocks SIGCHLD for itself).
    fn cat() -> std::path::PathBuf {
        std::env::var_os("PATH")
            .iter()
            .flat_map(std::env::split_paths)
            .map(|dir| dir.join("cat"))
            .find(|path| path.is_file())
            .expect("cat on PATH")
    }

    /// The SigBlk mask a child spawned from this thread starts with.
    fn child_sigblk(command: &mut Command) -> String {
        let out = command.arg("/proc/self/status").output().unwrap();
        assert!(out.status.success());
        String::from_utf8(out.stdout)
            .unwrap()
            .lines()
            .find_map(|line| line.strip_prefix("SigBlk:"))
            .expect("a SigBlk line")
            .trim()
            .to_string()
    }

    #[test]
    fn children_start_with_an_empty_signal_mask() {
        let _signals = SignalFd::new(&[
            Signal::Terminate,
            Signal::Interrupt,
            Signal::Hangup,
            Signal::Child,
        ])
        .unwrap();

        // Control: std's Command passes the blocked mask on — SIGHUP(1),
        // SIGINT(2), SIGTERM(15) and SIGCHLD(17) are bits 0, 1, 14 and 16.
        assert_eq!(child_sigblk(&mut Command::new(cat())), "0000000000014003");
        assert_eq!(
            child_sigblk(Command::new(cat()).unblock_signals()),
            "0000000000000000"
        );
    }
}

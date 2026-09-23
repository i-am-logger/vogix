//! systemd notifications (`sd_notify(3)`) without libsystemd.
//!
//! `$NOTIFY_SOCKET` names an `AF_UNIX` datagram socket: a filesystem path, or
//! an abstract name written with a leading '@'. A notification is one
//! datagram of newline-separated `KEY=VALUE` assignments. When the variable
//! is unset — the owner was started by hand — there is no notifier and
//! nothing is sent.

use std::ffi::OsStr;
use std::fmt;
use std::io;
use std::os::linux::net::SocketAddrExt;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::net::{SocketAddr, UnixDatagram};
use std::path::PathBuf;

/// The environment variable systemd sets for `Type=notify` services and for
/// any unit with `NotifyAccess=`.
pub const NOTIFY_SOCKET_ENV: &str = "NOTIFY_SOCKET";

/// Where notifications go, parsed from `$NOTIFY_SOCKET`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotifyTarget {
    /// A socket bound in the filesystem.
    Path(PathBuf),
    /// An abstract socket; the bytes are the name after the leading NUL.
    Abstract(Vec<u8>),
}

impl NotifyTarget {
    /// Parse a `$NOTIFY_SOCKET` value: `/…` is a path, `@…` an abstract name.
    pub fn parse(value: &OsStr) -> io::Result<Self> {
        let bytes = value.as_bytes();
        match bytes.first() {
            Some(b'/') => Ok(Self::Path(PathBuf::from(value))),
            Some(b'@') if bytes.len() > 1 => Ok(Self::Abstract(bytes[1..].to_vec())),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "{NOTIFY_SOCKET_ENV}={} is neither an absolute path nor an '@' abstract name",
                    value.to_string_lossy()
                ),
            )),
        }
    }

    fn socket_addr(&self) -> io::Result<SocketAddr> {
        match self {
            Self::Path(path) => SocketAddr::from_pathname(path),
            Self::Abstract(name) => SocketAddr::from_abstract_name(name),
        }
    }
}

/// A `STATUS=` text: one line. Newlines and other control characters, which
/// would start a new assignment or garble `systemctl status`, become spaces.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusLine(String);

impl StatusLine {
    pub fn new(text: impl Into<String>) -> Self {
        let text: String = text.into();
        Self(
            text.chars()
                .map(|c| if c.is_control() { ' ' } else { c })
                .collect(),
        )
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for StatusLine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// One assignment of a notification datagram.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Notification {
    /// `READY=1`: startup is complete (the unit leaves "activating").
    Ready,
    /// `STOPPING=1`: the service is shutting down.
    Stopping,
    /// `STATUS=…`: the line `systemctl status` shows.
    Status(StatusLine),
}

impl Notification {
    fn write_to(&self, out: &mut Vec<u8>) {
        match self {
            Self::Ready => out.extend_from_slice(b"READY=1"),
            Self::Stopping => out.extend_from_slice(b"STOPPING=1"),
            Self::Status(line) => {
                out.extend_from_slice(b"STATUS=");
                out.extend_from_slice(line.as_str().as_bytes());
            }
        }
    }
}

/// The datagram for `notifications`, assignments separated by newlines.
pub fn encode(notifications: &[Notification]) -> Vec<u8> {
    let mut out = Vec::new();
    for (i, notification) in notifications.iter().enumerate() {
        if i > 0 {
            out.push(b'\n');
        }
        notification.write_to(&mut out);
    }
    out
}

/// A sender of notifications to one target.
#[derive(Debug)]
pub struct Notifier {
    socket: UnixDatagram,
    addr: SocketAddr,
}

impl Notifier {
    /// The notifier `$NOTIFY_SOCKET` names, or `None` when it is unset.
    pub fn from_env() -> io::Result<Option<Self>> {
        match std::env::var_os(NOTIFY_SOCKET_ENV) {
            None => Ok(None),
            Some(value) => Self::to(&NotifyTarget::parse(&value)?).map(Some),
        }
    }

    /// A notifier for `target`, sending from an unbound datagram socket.
    pub fn to(target: &NotifyTarget) -> io::Result<Self> {
        Ok(Self {
            socket: UnixDatagram::unbound()?,
            addr: target.socket_addr()?,
        })
    }

    /// Send `notifications` as one datagram.
    pub fn send(&self, notifications: &[Notification]) -> io::Result<()> {
        let datagram = encode(notifications);
        let sent = self.socket.send_to_addr(&datagram, &self.addr)?;
        if sent != datagram.len() {
            return Err(io::Error::new(
                io::ErrorKind::WriteZero,
                format!("sent {sent} of {} notification bytes", datagram.len()),
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;
    use std::sync::atomic::{AtomicU32, Ordering};

    fn received(receiver: &UnixDatagram) -> Vec<u8> {
        let mut buf = [0u8; 4096];
        let n = receiver.recv(&mut buf).unwrap();
        buf[..n].to_vec()
    }

    #[test]
    fn parse_distinguishes_paths_and_abstract_names() {
        assert_eq!(
            NotifyTarget::parse(OsStr::new("/run/systemd/notify")).unwrap(),
            NotifyTarget::Path(PathBuf::from("/run/systemd/notify"))
        );
        assert_eq!(
            NotifyTarget::parse(OsStr::new("@/org/freedesktop/systemd1/notify/42")).unwrap(),
            NotifyTarget::Abstract(b"/org/freedesktop/systemd1/notify/42".to_vec())
        );
        for bad in ["", "@", "relative/notify", "vsock:2:1234"] {
            assert!(NotifyTarget::parse(OsStr::new(bad)).is_err(), "{bad:?}");
        }
        let non_utf8: OsString =
            std::os::unix::ffi::OsStringExt::from_vec(b"/run/\xffnotify".to_vec());
        assert!(matches!(
            NotifyTarget::parse(&non_utf8).unwrap(),
            NotifyTarget::Path(_)
        ));
    }

    #[test]
    fn status_lines_are_single_lines() {
        assert_eq!(
            StatusLine::new("protocol 6;\nREADY=1\tx").as_str(),
            "protocol 6; READY=1 x"
        );
    }

    #[test]
    fn a_path_socket_receives_the_exact_datagram() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("notify");
        let receiver = UnixDatagram::bind(&path).unwrap();

        let notifier = Notifier::to(&NotifyTarget::parse(path.as_os_str()).unwrap()).unwrap();
        notifier
            .send(&[
                Notification::Ready,
                Notification::Status(StatusLine::new(
                    "protocol 6; 3 controllers; dram-rgb confirmed x2",
                )),
            ])
            .unwrap();
        assert_eq!(
            received(&receiver),
            b"READY=1\nSTATUS=protocol 6; 3 controllers; dram-rgb confirmed x2"
        );

        notifier.send(&[Notification::Stopping]).unwrap();
        assert_eq!(received(&receiver), b"STOPPING=1");
    }

    #[test]
    fn an_abstract_socket_receives_the_exact_datagram() {
        static SEQ: AtomicU32 = AtomicU32::new(0);
        let name = format!(
            "vogix-notify-test-{}-{}",
            std::process::id(),
            SEQ.fetch_add(1, Ordering::Relaxed)
        );
        let receiver =
            UnixDatagram::bind_addr(&SocketAddr::from_abstract_name(name.as_bytes()).unwrap())
                .unwrap();

        let target = NotifyTarget::parse(OsStr::new(&format!("@{name}"))).unwrap();
        Notifier::to(&target)
            .unwrap()
            .send(&[Notification::Status(StatusLine::new(
                "waiting for owner palette",
            ))])
            .unwrap();
        assert_eq!(received(&receiver), b"STATUS=waiting for owner palette");
    }

    #[test]
    fn a_missing_socket_is_an_error_not_a_hang() {
        let dir = tempfile::tempdir().unwrap();
        let target = NotifyTarget::Path(dir.path().join("absent"));
        let err = Notifier::to(&target)
            .unwrap()
            .send(&[Notification::Ready])
            .unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::NotFound);
    }
}

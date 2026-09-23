//! hidraw hotplug from kernel uevents.
//!
//! The local owner re-runs a command device when a hidraw node of its USB
//! device appears (a plug-in, or a re-enumeration after resume). The kernel
//! announces every device on the `NETLINK_KOBJECT_UEVENT` socket's multicast
//! group 1, which any user may join: each datagram is `ACTION@DEVPATH\0`
//! followed by `KEY=VALUE\0` fields. A hidraw node's USB ids are its parent
//! HID device's `HID_ID`, read from `/sys<DEVPATH>/device/uevent`.
//!
//! [`HidrawMonitor::open`] binds the socket before it scans
//! `/sys/class/hidraw`, so a node is either in the scan or announced after
//! it — never missed between the two.

use super::config::HidrawMatch;
use crate::fsutil;
use std::ffi::OsString;
use std::fmt;
use std::fs::File;
use std::io;
use std::mem::MaybeUninit;
use std::os::fd::{AsFd, AsRawFd, BorrowedFd, FromRawFd, OwnedFd};
use std::path::{Component, Path, PathBuf};
use std::str::FromStr;

/// The kernel's uevent multicast group.
const KERNEL_GROUP: u32 = 1;

/// A uevent is at most a 2 KiB environment plus its header; anything larger
/// is truncated by the receive and skipped.
const MAX_UEVENT_BYTES: usize = 8192;

/// A sysfs `uevent` attribute file is at most a page.
const MAX_SYSFS_UEVENT_BYTES: u64 = 4096;

/// The kernel's `kobject_action` names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UeventAction {
    Add,
    Remove,
    Change,
    Move,
    Online,
    Offline,
    Bind,
    Unbind,
}

impl FromStr for UeventAction {
    type Err = UeventError;
    fn from_str(s: &str) -> Result<Self, UeventError> {
        Ok(match s {
            "add" => Self::Add,
            "remove" => Self::Remove,
            "change" => Self::Change,
            "move" => Self::Move,
            "online" => Self::Online,
            "offline" => Self::Offline,
            "bind" => Self::Bind,
            "unbind" => Self::Unbind,
            other => return Err(UeventError::UnknownAction(other.to_string())),
        })
    }
}

/// The fields of a kernel uevent the owner uses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Uevent<'a> {
    pub action: UeventAction,
    pub devpath: &'a str,
    pub subsystem: &'a str,
    pub devname: Option<&'a str>,
    pub seqnum: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum UeventError {
    #[error("not a kernel uevent: no ACTION@DEVPATH header")]
    NoHeader,
    #[error("uevent has no {0}")]
    Missing(&'static str),
    #[error("uevent field {0} is not UTF-8")]
    NotUtf8(&'static str),
    #[error("uevent field {0:?} has no '='")]
    MalformedField(String),
    #[error("uevent SEQNUM {0:?} is not a number")]
    BadSeqnum(String),
    #[error("uevent header {header:?} disagrees with its ACTION and DEVPATH")]
    HeaderMismatch { header: String },
    #[error("unknown uevent action {0:?}")]
    UnknownAction(String),
}

/// Parse one kernel uevent datagram. Only the fields the owner uses must be
/// UTF-8; other values (a device's name string, say) may hold any bytes.
pub fn parse_uevent<'a>(msg: &'a [u8]) -> Result<Uevent<'a>, UeventError> {
    let mut fields = msg.split(|&b| b == 0).filter(|f| !f.is_empty());
    let header = fields.next().ok_or(UeventError::NoHeader)?;
    let at = header
        .iter()
        .position(|&b| b == b'@')
        .ok_or(UeventError::NoHeader)?;
    let (header_action, header_devpath) = (&header[..at], &header[at + 1..]);

    let text = |key: &'static str, value: &'a [u8]| {
        std::str::from_utf8(value).map_err(|_| UeventError::NotUtf8(key))
    };
    let (mut action, mut devpath, mut subsystem, mut devname, mut seqnum) =
        (None, None, None, None, None);
    for field in fields {
        let eq = field
            .iter()
            .position(|&b| b == b'=')
            .ok_or_else(|| UeventError::MalformedField(String::from_utf8_lossy(field).into()))?;
        let (key, value) = (&field[..eq], &field[eq + 1..]);
        match key {
            b"ACTION" => action = Some(text("ACTION", value)?),
            b"DEVPATH" => devpath = Some(text("DEVPATH", value)?),
            b"SUBSYSTEM" => subsystem = Some(text("SUBSYSTEM", value)?),
            b"DEVNAME" => devname = Some(text("DEVNAME", value)?),
            b"SEQNUM" => {
                let raw = text("SEQNUM", value)?;
                seqnum = Some(
                    raw.parse::<u64>()
                        .map_err(|_| UeventError::BadSeqnum(raw.to_string()))?,
                );
            }
            _ => {}
        }
    }
    let action = action.ok_or(UeventError::Missing("ACTION"))?;
    let devpath = devpath.ok_or(UeventError::Missing("DEVPATH"))?;
    let subsystem = subsystem.ok_or(UeventError::Missing("SUBSYSTEM"))?;
    if header_action != action.as_bytes() || header_devpath != devpath.as_bytes() {
        return Err(UeventError::HeaderMismatch {
            header: String::from_utf8_lossy(header).into(),
        });
    }
    Ok(Uevent {
        action: action.parse()?,
        devpath,
        subsystem,
        devname,
        seqnum,
    })
}

/// A HID device's bus and ids, as its `HID_ID=BBBB:VVVVVVVV:PPPPPPPP`
/// uevent field states them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HidId {
    pub bus: u16,
    pub vendor: u32,
    pub product: u32,
}

impl HidId {
    /// Parse the value of a `HID_ID` field.
    pub fn parse(value: &str) -> Option<Self> {
        fn hex<const DIGITS: usize>(s: &str) -> Option<u32> {
            (s.len() == DIGITS && s.bytes().all(|b| b.is_ascii_hexdigit()))
                .then(|| u32::from_str_radix(s, 16).ok())
                .flatten()
        }
        let mut parts = value.split(':');
        let bus = hex::<4>(parts.next()?)?;
        let vendor = hex::<8>(parts.next()?)?;
        let product = hex::<8>(parts.next()?)?;
        if parts.next().is_some() {
            return None;
        }
        Some(Self {
            bus: u16::try_from(bus).ok()?,
            vendor,
            product,
        })
    }

    /// The `HID_ID` of a sysfs `uevent` file (`KEY=VALUE` lines).
    pub fn from_uevent_file(text: &str) -> Option<Self> {
        text.lines()
            .find_map(|line| line.strip_prefix("HID_ID="))
            .and_then(Self::parse)
    }

    pub fn matches(&self, wanted: &HidrawMatch) -> bool {
        self.vendor == u32::from(wanted.vendor_id.value())
            && self.product == u32::from(wanted.product_id.value())
    }
}

impl fmt::Display for HidId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04x}:{:04x}", self.vendor, self.product)
    }
}

/// A hidraw node and the ids of the HID device behind it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hidraw {
    /// The node name, e.g. `hidraw3`.
    pub node: String,
    pub hid: HidId,
}

/// hidraw nodes announced since the last drain.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Arrivals {
    pub devices: Vec<Hidraw>,
    /// The socket's queue overflowed, so announcements were lost;
    /// `devices` is then a fresh scan of every present node.
    pub rescanned: bool,
}

/// The kernel uevent socket, filtered to hidraw arrivals.
#[derive(Debug)]
pub struct HidrawMonitor {
    socket: OwnedFd,
    sys: PathBuf,
    buf: Box<[u8]>,
}

impl HidrawMonitor {
    /// Join the kernel uevent group, then scan `<sys>/class/hidraw`. Returns
    /// the monitor and the nodes present at the scan.
    pub fn open(sys: &Path) -> io::Result<(Self, Vec<Hidraw>)> {
        let socket = kernel_uevent_socket()?;
        let present = scan_hidraw(sys)?;
        Ok((
            Self {
                socket,
                sys: sys.to_path_buf(),
                buf: vec![0; MAX_UEVENT_BYTES].into_boxed_slice(),
            },
            present,
        ))
    }

    /// Read every queued uevent and return the hidraw nodes that were added.
    pub fn drain(&mut self) -> io::Result<Arrivals> {
        let mut arrivals = Arrivals::default();
        let mut overflowed = false;
        loop {
            // SAFETY: an all-zero sockaddr_nl is valid.
            let mut source: libc::sockaddr_nl = unsafe { std::mem::zeroed() };
            let mut source_len = std::mem::size_of::<libc::sockaddr_nl>() as libc::socklen_t;
            // SAFETY: `buf` and `source` are valid for writes of the sizes given.
            let n = unsafe {
                libc::recvfrom(
                    self.socket.as_raw_fd(),
                    self.buf.as_mut_ptr().cast(),
                    self.buf.len(),
                    libc::MSG_TRUNC,
                    (&mut source as *mut libc::sockaddr_nl).cast(),
                    &mut source_len,
                )
            };
            if n < 0 {
                let err = io::Error::last_os_error();
                match err.raw_os_error() {
                    Some(libc::EAGAIN) => break,
                    Some(libc::EINTR) => continue,
                    Some(libc::ENOBUFS) => {
                        overflowed = true;
                        continue;
                    }
                    _ => return Err(err),
                }
            }
            let n = n as usize;
            if n > self.buf.len() {
                log::warn!("skipped a {n}-byte uevent larger than the receive buffer");
                continue;
            }
            if let Some(hidraw) = self.arrival(&self.buf[..n], source.nl_pid) {
                arrivals.devices.push(hidraw);
            }
        }
        if overflowed {
            log::warn!("the uevent queue overflowed; rescanning hidraw nodes");
            arrivals = Arrivals {
                devices: scan_hidraw(&self.sys)?,
                rescanned: true,
            };
        }
        Ok(arrivals)
    }

    /// The hidraw node a datagram announces, if it is a kernel hidraw `add`
    /// whose HID parent is still there to be read.
    fn arrival(&self, datagram: &[u8], sender_port: u32) -> Option<Hidraw> {
        if sender_port != 0 {
            log::debug!("ignored a uevent from port {sender_port}, not the kernel");
            return None;
        }
        let event = match parse_uevent(datagram) {
            Ok(event) => event,
            Err(e) => {
                log::debug!("ignored a uevent: {e}");
                return None;
            }
        };
        if event.action != UeventAction::Add || event.subsystem != "hidraw" {
            return None;
        }
        let node = event
            .devname
            .or_else(|| event.devpath.rsplit('/').next())?
            .to_string();
        match hid_id_of(&self.sys, event.devpath) {
            Ok(hid) => Some(Hidraw { node, hid }),
            Err(e) => {
                log::debug!("{node}: no HID ids ({e})");
                None
            }
        }
    }
}

impl AsFd for HidrawMonitor {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.socket.as_fd()
    }
}

/// A non-blocking `NETLINK_KOBJECT_UEVENT` socket joined to the kernel group.
fn kernel_uevent_socket() -> io::Result<OwnedFd> {
    // SAFETY: plain socket(2) call.
    let fd = unsafe {
        libc::socket(
            libc::AF_NETLINK,
            libc::SOCK_DGRAM | libc::SOCK_CLOEXEC | libc::SOCK_NONBLOCK,
            libc::NETLINK_KOBJECT_UEVENT,
        )
    };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: socket returned a fresh fd that nothing else owns.
    let socket = unsafe { OwnedFd::from_raw_fd(fd) };
    let mut addr = MaybeUninit::<libc::sockaddr_nl>::zeroed();
    // SAFETY: all-zero is a valid sockaddr_nl; the fields are then set.
    let addr = unsafe {
        let a = addr.as_mut_ptr();
        (*a).nl_family = libc::AF_NETLINK as libc::sa_family_t;
        (*a).nl_groups = KERNEL_GROUP;
        addr.assume_init()
    };
    // SAFETY: `addr` is a valid sockaddr_nl of the length given.
    let rc = unsafe {
        libc::bind(
            socket.as_raw_fd(),
            (&addr as *const libc::sockaddr_nl).cast(),
            std::mem::size_of::<libc::sockaddr_nl>() as libc::socklen_t,
        )
    };
    if rc != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(socket)
}

/// The HID ids behind the hidraw device at `devpath` (a DEVPATH, relative to
/// the sysfs root `sys`).
fn hid_id_of(sys: &Path, devpath: &str) -> io::Result<HidId> {
    let relative = Path::new(devpath)
        .strip_prefix("/")
        .map_err(|_| invalid(format!("DEVPATH {devpath:?} is not absolute")))?;
    if !relative
        .components()
        .all(|c| matches!(c, Component::Normal(_)))
    {
        return Err(invalid(format!("DEVPATH {devpath:?} is not normalized")));
    }
    read_hid_id(&sys.join(relative).join("device/uevent"))
}

fn read_hid_id(uevent_file: &Path) -> io::Result<HidId> {
    let bytes = fsutil::read_capped(&mut File::open(uevent_file)?, MAX_SYSFS_UEVENT_BYTES)?;
    let text = String::from_utf8_lossy(&bytes);
    HidId::from_uevent_file(&text)
        .ok_or_else(|| invalid(format!("{} has no HID_ID", uevent_file.display())))
}

fn invalid(message: String) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

/// Every hidraw node under `<sys>/class/hidraw`, in name order. No such
/// directory (the hidraw class is not registered) is no nodes; a node that
/// vanishes during the scan is skipped.
pub fn scan_hidraw(sys: &Path) -> io::Result<Vec<Hidraw>> {
    let class = sys.join("class/hidraw");
    let mut names: Vec<OsString> = match std::fs::read_dir(&class) {
        Ok(entries) => entries
            .map(|entry| entry.map(|e| e.file_name()))
            .collect::<io::Result<_>>()?,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e),
    };
    names.sort();
    let mut found = Vec::new();
    for name in names {
        let node = name.to_string_lossy().into_owned();
        match read_hid_id(&class.join(&name).join("device/uevent")) {
            Ok(hid) => found.push(Hidraw { node, hid }),
            Err(e) => log::debug!("{node}: no HID ids ({e})"),
        }
    }
    Ok(found)
}

#[cfg(test)]
mod tests {
    //! The `tests/fixtures/uevent` datagrams are real kernel uevents,
    //! captured byte for byte from a `NETLINK_KOBJECT_UEVENT` socket in a
    //! NixOS test VM while QEMU hot-added a `usb-kbd` (USB 0627:0001) on an
    //! xHCI bus; every one was sent by the kernel (port 0).
    //! `hid-device-uevent` is that VM's `/sys/class/hidraw/hidraw0/device/uevent`.
    use super::*;
    use crate::machine::types::UsbId;
    use std::os::unix::fs::symlink;

    const HIDRAW_ADD: &[u8] = include_bytes!("../../tests/fixtures/uevent/hidraw-add.bin");
    const HID_ADD: &[u8] = include_bytes!("../../tests/fixtures/uevent/hid-add.bin");
    const HID_BIND: &[u8] = include_bytes!("../../tests/fixtures/uevent/hid-bind.bin");
    const INPUT_ADD: &[u8] = include_bytes!("../../tests/fixtures/uevent/input-add.bin");
    const USB_ADD: &[u8] = include_bytes!("../../tests/fixtures/uevent/usb-add.bin");
    const FAUX_CHANGE: &[u8] = include_bytes!("../../tests/fixtures/uevent/faux-change.bin");
    const HID_DEVICE_UEVENT: &str = include_str!("../../tests/fixtures/uevent/hid-device-uevent");

    const KBD_HID: &str = "devices/pci0000:00/0000:00:0a.0/usb2/2-1/2-1:1.0/0003:0627:0001.0002";

    fn qemu_hid() -> HidrawMatch {
        HidrawMatch {
            vendor_id: UsbId::new(0x0627),
            product_id: UsbId::new(0x0001),
        }
    }

    /// A sysfs tree shaped like the VM's: the HID device, its hidraw1 child
    /// with the `device` link back to it, and the class link.
    fn sysfs_with_hidraw1() -> tempfile::TempDir {
        let sys = tempfile::tempdir().unwrap();
        let hid = sys.path().join(KBD_HID);
        std::fs::create_dir_all(hid.join("hidraw/hidraw1")).unwrap();
        std::fs::write(hid.join("uevent"), HID_DEVICE_UEVENT).unwrap();
        symlink("../..", hid.join("hidraw/hidraw1/device")).unwrap();
        std::fs::create_dir_all(sys.path().join("class/hidraw")).unwrap();
        symlink(
            format!("../../{KBD_HID}/hidraw/hidraw1"),
            sys.path().join("class/hidraw/hidraw1"),
        )
        .unwrap();
        sys
    }

    #[test]
    fn a_real_hidraw_add_parses() {
        let event = parse_uevent(HIDRAW_ADD).unwrap();
        assert_eq!(event.action, UeventAction::Add);
        assert_eq!(event.subsystem, "hidraw");
        assert_eq!(event.devname, Some("hidraw1"));
        assert_eq!(event.seqnum, Some(2195));
        assert_eq!(event.devpath, format!("/{KBD_HID}/hidraw/hidraw1"));
    }

    #[test]
    fn other_real_events_parse_with_their_own_subsystem_and_action() {
        let cases: [(&[u8], UeventAction, &str); 5] = [
            (HID_ADD, UeventAction::Add, "hid"),
            (HID_BIND, UeventAction::Bind, "hid"),
            (INPUT_ADD, UeventAction::Add, "input"),
            (USB_ADD, UeventAction::Add, "usb"),
            (FAUX_CHANGE, UeventAction::Change, "faux"),
        ];
        for (bytes, action, subsystem) in cases {
            let event = parse_uevent(bytes).unwrap();
            assert_eq!((event.action, event.subsystem), (action, subsystem));
        }
    }

    #[test]
    fn malformed_datagrams_are_errors() {
        assert_eq!(parse_uevent(b""), Err(UeventError::NoHeader));
        assert_eq!(
            parse_uevent(b"libudev\0\xfe\xed\xca\xfe"),
            Err(UeventError::NoHeader)
        );
        assert_eq!(
            parse_uevent(b"add@/devices/x\0DEVPATH=/devices/x\0SUBSYSTEM=hidraw\0"),
            Err(UeventError::Missing("ACTION"))
        );
        assert!(matches!(
            parse_uevent(b"add@/devices/x\0ACTION=add\0DEVPATH=/devices/y\0SUBSYSTEM=hidraw\0"),
            Err(UeventError::HeaderMismatch { .. })
        ));
        assert!(matches!(
            parse_uevent(b"eat@/d\0ACTION=eat\0DEVPATH=/d\0SUBSYSTEM=hidraw\0"),
            Err(UeventError::UnknownAction(_))
        ));
        assert!(matches!(
            parse_uevent(b"add@/d\0ACTION=add\0DEVPATH=/d\0SUBSYSTEM=x\0SEQNUM=12a\0"),
            Err(UeventError::BadSeqnum(_))
        ));
        assert!(matches!(
            parse_uevent(b"add@/d\0ACTION=add\0garbage\0"),
            Err(UeventError::MalformedField(_))
        ));
        // A value the owner does not use may hold any bytes.
        assert!(
            parse_uevent(b"add@/d\0ACTION=add\0DEVPATH=/d\0SUBSYSTEM=hid\0HID_NAME=\xff\0").is_ok()
        );
    }

    #[test]
    fn every_truncation_of_a_real_event_is_handled_without_panic() {
        for len in 0..HIDRAW_ADD.len() {
            let _ = parse_uevent(&HIDRAW_ADD[..len]);
        }
    }

    #[test]
    fn hid_ids_parse_from_the_real_parent_uevent_file() {
        let hid = HidId::from_uevent_file(HID_DEVICE_UEVENT).unwrap();
        assert_eq!(
            hid,
            HidId {
                bus: 3,
                vendor: 0x0627,
                product: 0x0001
            }
        );
        assert!(hid.matches(&qemu_hid()));
        assert!(!hid.matches(&HidrawMatch {
            vendor_id: UsbId::new(0x1e71),
            product_id: UsbId::new(0x3012),
        }));
        assert_eq!(hid.to_string(), "0627:0001");
    }

    #[test]
    fn hid_id_values_are_strict() {
        assert_eq!(
            HidId::parse("0003:00001E71:00003012"),
            Some(HidId {
                bus: 3,
                vendor: 0x1e71,
                product: 0x3012
            })
        );
        for bad in [
            "",
            "0003:00001E71",
            "0003:1E71:3012",
            "0003:00001E71:00003012:0",
            "0003:+0001E71:00003012",
            "00003:00001E71:00003012",
        ] {
            assert_eq!(HidId::parse(bad), None, "{bad:?}");
        }
        assert_eq!(HidId::from_uevent_file("DRIVER=hid-generic\n"), None);
    }

    #[test]
    fn coldplug_finds_the_node_through_the_class_link() {
        let sys = sysfs_with_hidraw1();
        assert_eq!(
            scan_hidraw(sys.path()).unwrap(),
            [Hidraw {
                node: "hidraw1".into(),
                hid: HidId {
                    bus: 3,
                    vendor: 0x0627,
                    product: 0x0001
                }
            }]
        );
        let empty = tempfile::tempdir().unwrap();
        assert!(scan_hidraw(empty.path()).unwrap().is_empty());
    }

    #[test]
    fn a_kernel_hidraw_add_resolves_its_parent_ids_and_nothing_else_does() {
        let sys = sysfs_with_hidraw1();
        let (monitor, present) = HidrawMonitor::open(sys.path()).unwrap();
        assert_eq!(present.len(), 1);

        let hidraw = monitor.arrival(HIDRAW_ADD, 0).unwrap();
        assert_eq!(hidraw.node, "hidraw1");
        assert!(hidraw.hid.matches(&qemu_hid()));

        assert_eq!(
            monitor.arrival(HIDRAW_ADD, 4242),
            None,
            "not from the kernel"
        );
        for other in [HID_ADD, HID_BIND, INPUT_ADD, USB_ADD, FAUX_CHANGE] {
            assert_eq!(monitor.arrival(other, 0), None);
        }

        // The node's parent is gone (unplugged before the read): skipped.
        let empty = tempfile::tempdir().unwrap();
        let (gone, _) = HidrawMonitor::open(empty.path()).unwrap();
        assert_eq!(gone.arrival(HIDRAW_ADD, 0), None);
    }

    #[test]
    fn a_devpath_cannot_climb_out_of_sysfs() {
        let sys = tempfile::tempdir().unwrap();
        for devpath in [
            "/devices/../../etc",
            "relative/hidraw0",
            "/devices/hidraw/..",
        ] {
            let err = hid_id_of(sys.path(), devpath).unwrap_err();
            assert_eq!(err.kind(), io::ErrorKind::InvalidData, "{devpath}");
        }
    }

    /// The real kernel socket: joining the uevent group needs no privilege,
    /// and draining whatever the kernel has queued is not an error.
    #[test]
    fn the_kernel_uevent_socket_binds_unprivileged_and_drains() {
        let sys = tempfile::tempdir().unwrap();
        let (mut monitor, present) = HidrawMonitor::open(sys.path()).unwrap();
        assert!(present.is_empty());
        let arrivals = monitor.drain().unwrap();
        assert!(!arrivals.rescanned);
        // Arrivals resolve against this empty sysfs root, so none can match.
        assert!(arrivals.devices.is_empty());
        // SAFETY: fcntl F_GETFL on an fd this monitor owns.
        let flags = unsafe { libc::fcntl(monitor.as_fd().as_raw_fd(), libc::F_GETFL) };
        assert_ne!(flags & libc::O_NONBLOCK, 0);
    }
}

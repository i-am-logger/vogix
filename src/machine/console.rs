//! The kernel's VT palette: the 16 colours every virtual console draws its
//! text with.
//!
//! `GIO_CMAP` reads the kernel's default palette (the values behind
//! `/sys/module/vt/parameters/default_{red,grn,blu}`); `PIO_CMAP` replaces it
//! and the palette of every allocated console. A console in text mode on
//! screen is repainted at once; one in graphics mode (a compositor) shows the
//! new colours at its next switch to text. Both ioctls work on any VT fd;
//! `PIO_CMAP` needs `CAP_SYS_TTY_CONFIG` unless the caller's controlling tty
//! is that VT.
//!
//! The local owner opens [`CONSOLE_DEVICE`] afresh for each reconcile,
//! compares, and writes only when the kernel holds other colours. It never
//! switches VTs.

use super::types::Rgb;
use std::fs::{File, OpenOptions};
use std::io;
use std::os::fd::AsRawFd;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

/// The foreground virtual console.
pub const CONSOLE_DEVICE: &str = "/dev/tty0";

/// `linux/kd.h`: read the default colour map (48 bytes).
const GIO_CMAP: libc::Ioctl = 0x4B70;
/// `linux/kd.h`: write the default colour map (48 bytes).
const PIO_CMAP: libc::Ioctl = 0x4B71;

/// A console colour map as the kernel exchanges it: 16 `r, g, b` byte
/// triplets in ANSI colour order (black, red, green, yellow, blue, magenta,
/// cyan, white, then the bright eight).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ColourMap([u8; 48]);

impl ColourMap {
    pub fn from_colours(colours: &[Rgb; 16]) -> Self {
        let mut bytes = [0u8; 48];
        for (triplet, colour) in bytes.chunks_exact_mut(3).zip(colours) {
            let (r, g, b) = colour.channels();
            triplet.copy_from_slice(&[r, g, b]);
        }
        Self(bytes)
    }

    /// The colour at ANSI index `index` (0..16).
    #[cfg(test)]
    pub fn colour(&self, index: usize) -> Rgb {
        let t = &self.0[index * 3..index * 3 + 3];
        Rgb::new(t[0], t[1], t[2])
    }

    #[cfg(test)]
    pub fn as_bytes(&self) -> &[u8; 48] {
        &self.0
    }
}

/// What a reconcile did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reconciled {
    /// The kernel already held these colours; nothing was written.
    AlreadyCurrent,
    /// The colours were written with `PIO_CMAP`.
    Written,
}

#[derive(Debug, thiserror::Error)]
pub enum ConsoleError {
    #[error("cannot open {path}: {source}")]
    Open { path: PathBuf, source: io::Error },
    #[error("GIO_CMAP on {path}: {source}")]
    Read { path: PathBuf, source: io::Error },
    #[error("PIO_CMAP on {path}: {source}")]
    Write { path: PathBuf, source: io::Error },
}

/// Make the kernel's VT palette `wanted`: open `device`, read the current
/// map, and write `wanted` only when it differs.
pub fn reconcile(device: &Path, wanted: &ColourMap) -> Result<Reconciled, ConsoleError> {
    let tty = open(device).map_err(|source| ConsoleError::Open {
        path: device.to_path_buf(),
        source,
    })?;
    let current = read_map(&tty).map_err(|source| ConsoleError::Read {
        path: device.to_path_buf(),
        source,
    })?;
    if current == *wanted {
        return Ok(Reconciled::AlreadyCurrent);
    }
    write_map(&tty, wanted).map_err(|source| ConsoleError::Write {
        path: device.to_path_buf(),
        source,
    })?;
    Ok(Reconciled::Written)
}

/// `open(device, O_RDWR | O_NOCTTY | O_CLOEXEC)`: a VT fd that does not
/// become the owner's controlling terminal.
fn open(device: &Path) -> io::Result<File> {
    OpenOptions::new()
        .read(true)
        .write(true)
        .custom_flags(libc::O_NOCTTY)
        .open(device)
}

fn read_map(tty: &File) -> io::Result<ColourMap> {
    let mut map = [0u8; 48];
    // SAFETY: GIO_CMAP writes exactly 48 bytes into the buffer it is given,
    // which is valid for writes of that size for the call.
    let rc = unsafe { libc::ioctl(tty.as_raw_fd(), GIO_CMAP, map.as_mut_ptr()) };
    if rc < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(ColourMap(map))
}

fn write_map(tty: &File, map: &ColourMap) -> io::Result<()> {
    // SAFETY: PIO_CMAP reads exactly 48 bytes from the buffer it is given,
    // which is valid for reads of that size for the call.
    let rc = unsafe { libc::ioctl(tty.as_raw_fd(), PIO_CMAP, map.0.as_ptr()) };
    if rc < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn palette() -> [Rgb; 16] {
        std::array::from_fn(|i| {
            let i = i as u8;
            Rgb::new(i, 0x40 | i, 0x80 | i)
        })
    }

    #[test]
    fn the_map_is_sixteen_rgb_triplets_in_ansi_order() {
        let map = ColourMap::from_colours(&palette());
        let bytes = map.as_bytes();
        assert_eq!(bytes.len(), 48);
        assert_eq!(&bytes[..6], &[0x00, 0x40, 0x80, 0x01, 0x41, 0x81]);
        assert_eq!(&bytes[45..], &[0x0f, 0x4f, 0x8f]);
        for (i, colour) in palette().iter().enumerate() {
            assert_eq!(map.colour(i), *colour);
        }
    }

    #[test]
    fn a_file_that_is_not_a_vt_is_refused_by_the_read() {
        // /dev/null opens read-write but is no tty: GIO_CMAP fails with
        // ENOTTY and nothing is written.
        let map = ColourMap::from_colours(&palette());
        match reconcile(Path::new("/dev/null"), &map) {
            Err(ConsoleError::Read { path, source }) => {
                assert_eq!(path, Path::new("/dev/null"));
                assert_eq!(source.raw_os_error(), Some(libc::ENOTTY));
            }
            other => panic!("expected a GIO_CMAP error, got {other:?}"),
        }
    }

    #[test]
    fn a_missing_device_is_an_open_error_naming_it() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("tty0");
        let err = reconcile(&path, &ColourMap::from_colours(&palette())).unwrap_err();
        assert!(matches!(err, ConsoleError::Open { .. }));
        assert!(err.to_string().contains(&path.display().to_string()));
    }
}

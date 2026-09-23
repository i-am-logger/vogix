//! How a machine owner process ends. The exit status is part of its unit's
//! contract: both owner units restart on failure, except for
//! [`OwnerExit::Config`], which they list in `RestartPreventExitStatus=`.
//! A cause both owners meet has its exit defined here, once.

use std::io;

/// A machine owner's exit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OwnerExit {
    /// Stopped by SIGTERM or SIGINT: status 0.
    Stopped,
    /// The machine config, the drop zone or the unit's environment is
    /// unusable, and a restart would meet the same: status 78 (`EX_CONFIG`).
    Config,
    /// A fresh start can succeed: status 75 (`EX_TEMPFAIL`). Both owners end
    /// so when the drop zone is removed or moved
    /// ([`OwnerExit::drop_zone_gone`]); the local owner when an event source
    /// fails; the OpenRGB owner when OpenRGB refuses or closes the
    /// connection.
    TempFail,
}

impl OwnerExit {
    pub const fn code(self) -> i32 {
        match self {
            Self::Stopped => 0,
            Self::Config => 78,
            Self::TempFail => 75,
        }
    }

    /// The drop zone's watch reported the zone removed or moved away. What is
    /// gone is the watch, not necessarily a zone: a directory renamed into
    /// its place or re-created by tmpfiles is there for the next start, which
    /// resolves the path again and ends with [`OwnerExit::Config`] when it
    /// finds no directory there
    /// ([`OwnerExit::drop_zone_unwatchable`]).
    pub const fn drop_zone_gone() -> Self {
        Self::TempFail
    }

    /// The drop zone cannot be watched at start. No directory at its path,
    /// or one the owner may not read, is the machine's configuration, which
    /// a restart meets again; any other error (an inotify limit reached) can
    /// pass.
    pub fn drop_zone_unwatchable(error: &io::Error) -> Self {
        match error.kind() {
            io::ErrorKind::NotFound
            | io::ErrorKind::NotADirectory
            | io::ErrorKind::PermissionDenied => Self::Config,
            _ => Self::TempFail,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_codes_are_sysexits() {
        // sysexits.h: EX_TEMPFAIL 75, EX_CONFIG 78.
        assert_eq!(OwnerExit::Stopped.code(), 0);
        assert_eq!(OwnerExit::Config.code(), 78);
        assert_eq!(OwnerExit::TempFail.code(), 75);
    }

    #[test]
    fn a_drop_zone_that_went_away_is_temporary_and_a_start_without_one_is_config() {
        assert_eq!(OwnerExit::drop_zone_gone(), OwnerExit::TempFail);
        for kind in [
            io::ErrorKind::NotFound,
            io::ErrorKind::NotADirectory,
            io::ErrorKind::PermissionDenied,
        ] {
            assert_eq!(
                OwnerExit::drop_zone_unwatchable(&io::Error::from(kind)),
                OwnerExit::Config,
                "{kind:?}"
            );
        }
        // ENOSPC: the user's inotify watch limit.
        assert_eq!(
            OwnerExit::drop_zone_unwatchable(&io::Error::from_raw_os_error(libc::ENOSPC)),
            OwnerExit::TempFail
        );
    }

    #[test]
    fn a_start_on_the_real_filesystem_without_the_zone_is_config() {
        use crate::machine::reactor::DropZoneWatch;
        let dir = tempfile::TempDir::new().unwrap();
        let missing = dir.path().join("machine");
        let e = DropZoneWatch::new(&missing, std::ffi::OsStr::new("palette.json"))
            .expect_err("no directory to watch");
        assert_eq!(OwnerExit::drop_zone_unwatchable(&e), OwnerExit::Config);
        let file = dir.path().join("file");
        std::fs::write(&file, b"").unwrap();
        let e = DropZoneWatch::new(&file, std::ffi::OsStr::new("palette.json"))
            .expect_err("a file is not a drop zone");
        assert_eq!(OwnerExit::drop_zone_unwatchable(&e), OwnerExit::Config);
    }
}

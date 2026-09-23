//! How a machine owner process ends. The exit status is part of its unit's
//! contract: systemd restarts on failure, except for [`OwnerExit::Config`],
//! which the unit lists in `RestartPreventExitStatus=`.

/// A machine owner's exit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OwnerExit {
    /// Stopped by SIGTERM or SIGINT: status 0.
    Stopped,
    /// The machine config, the drop zone or the unit's environment is
    /// unusable, and a restart would meet the same: status 78 (`EX_CONFIG`).
    Config,
    /// An event source failed or the drop zone went away; a fresh start
    /// rebuilds them: status 75 (`EX_TEMPFAIL`).
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
}

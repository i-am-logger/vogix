//! Atomic replacement of files that other processes read.
//!
//! Every file vogix hands to another process — the greeter drop zone, the
//! machine palette, an owner's status file — is written the same way: a fresh
//! temp file beside the destination is filled, given its mode, fsynced and
//! renamed over the destination. Readers therefore see the old file or the new
//! one, never a torn one, and a destination that holds exactly the intended
//! bytes is left alone, so its watchers see no event.

use std::fs::{self, OpenOptions};
use std::io::{self, Read, Write};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};
use std::path::Path;

/// What [`write_atomic`] did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriteOutcome {
    /// A new file was renamed into place.
    Written,
    /// The destination already was a regular file owned by this user, with
    /// this mode and exactly these bytes; nothing was written.
    Unchanged,
}

/// Replace `dest` with `contents` at permission `mode`, atomically.
///
/// The destination is compared first (opened with `O_NOFOLLOW`): when it is
/// already a regular file owned by the effective user, with permission bits
/// equal to `mode` and exactly `contents`, nothing is written and
/// [`WriteOutcome::Unchanged`] is returned.
///
/// Otherwise `.<name>.tmp.<pid>` is created beside it with `O_EXCL` (which
/// never follows a symlink), `fchmod`ed to `mode` (independent of the umask),
/// filled, fsynced and renamed over `dest`. `rename` replaces a symlink at
/// `dest` rather than writing through it, so a planted link cannot redirect
/// the write. On any failure after the temp file was created it is removed.
pub fn write_atomic(dest: &Path, contents: &[u8], mode: u32) -> io::Result<WriteOutcome> {
    if holds_exactly(dest, contents, mode) {
        return Ok(WriteOutcome::Unchanged);
    }

    let name = dest.file_name().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{} names no file", dest.display()),
        )
    })?;
    let dir = match dest.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent,
        _ => Path::new("."),
    };
    let tmp = dir.join(format!(
        ".{}.tmp.{}",
        name.to_string_lossy(),
        std::process::id()
    ));

    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW)
        .open(&tmp)?;
    let result = file
        .set_permissions(fs::Permissions::from_mode(mode))
        .and_then(|()| file.write_all(contents))
        .and_then(|()| file.sync_all())
        .and_then(|()| fs::rename(&tmp, dest));
    if result.is_err() {
        // The temp file is ours (O_EXCL created it); a failed replace must
        // not leave it behind in a directory other processes watch.
        let _ = fs::remove_file(&tmp);
    }
    result.map(|()| WriteOutcome::Written)
}

/// Whether `dest` is already exactly what [`write_atomic`] would leave there.
/// Anything that cannot be proven identical — absent, a symlink, unreadable,
/// another owner, another mode, other bytes — is "not identical".
fn holds_exactly(dest: &Path, contents: &[u8], mode: u32) -> bool {
    let Ok(file) = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_NOCTTY)
        .open(dest)
    else {
        return false;
    };
    let Ok(meta) = file.metadata() else {
        return false;
    };
    // SAFETY: geteuid has no preconditions and cannot fail.
    let euid = unsafe { libc::geteuid() };
    if !meta.is_file()
        || meta.uid() != euid
        || meta.permissions().mode() & 0o7777 != mode
        || meta.len() != contents.len() as u64
    {
        return false;
    }
    let mut existing = Vec::with_capacity(contents.len());
    // Read one byte past the expected length so a file that grew after the
    // fstat still compares unequal.
    file.take(contents.len() as u64 + 1)
        .read_to_end(&mut existing)
        .is_ok_and(|_| existing == contents)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::symlink;

    fn mode_of(path: &Path) -> u32 {
        fs::symlink_metadata(path).unwrap().permissions().mode() & 0o7777
    }

    fn ino_of(path: &Path) -> u64 {
        fs::symlink_metadata(path).unwrap().ino()
    }

    fn leftovers(dir: &Path) -> Vec<String> {
        fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .filter(|n| n.contains(".tmp."))
            .collect()
    }

    #[test]
    fn writes_the_bytes_at_the_requested_mode_regardless_of_umask() {
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("palette.json");
        assert_eq!(
            write_atomic(&dest, b"{}", 0o644).unwrap(),
            WriteOutcome::Written
        );
        assert_eq!(fs::read(&dest).unwrap(), b"{}");
        assert_eq!(mode_of(&dest), 0o644);

        let shared = dir.path().join("theme.json");
        write_atomic(&shared, b"x", 0o664).unwrap();
        assert_eq!(mode_of(&shared), 0o664, "group-writable survives the umask");
        assert!(leftovers(dir.path()).is_empty());
    }

    #[test]
    fn identical_bytes_are_not_rewritten() {
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("palette.json");
        write_atomic(&dest, b"same", 0o644).unwrap();
        let before = ino_of(&dest);
        assert_eq!(
            write_atomic(&dest, b"same", 0o644).unwrap(),
            WriteOutcome::Unchanged
        );
        assert_eq!(ino_of(&dest), before, "no rename happened");

        assert_eq!(
            write_atomic(&dest, b"different", 0o644).unwrap(),
            WriteOutcome::Written
        );
        assert_ne!(ino_of(&dest), before, "a new file was renamed in");
        assert_eq!(fs::read(&dest).unwrap(), b"different");
    }

    #[test]
    fn same_bytes_with_another_mode_are_rewritten_at_the_requested_mode() {
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("palette.json");
        write_atomic(&dest, b"same", 0o600).unwrap();
        assert_eq!(
            write_atomic(&dest, b"same", 0o644).unwrap(),
            WriteOutcome::Written
        );
        assert_eq!(mode_of(&dest), 0o644);
    }

    #[test]
    fn a_prefix_of_the_intended_bytes_is_not_identical() {
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("f");
        write_atomic(&dest, b"abc", 0o644).unwrap();
        assert_eq!(
            write_atomic(&dest, b"abcd", 0o644).unwrap(),
            WriteOutcome::Written
        );
        assert_eq!(fs::read(&dest).unwrap(), b"abcd");
    }

    #[test]
    fn a_symlink_at_the_destination_is_replaced_not_followed() {
        let dir = tempfile::tempdir().unwrap();
        let victim = dir.path().join("victim");
        fs::write(&victim, b"keep").unwrap();
        let dest = dir.path().join("palette.json");
        symlink(&victim, &dest).unwrap();

        // Identical bytes behind the link still count as "not identical":
        // the link itself must go.
        assert_eq!(
            write_atomic(&dest, b"keep", 0o644).unwrap(),
            WriteOutcome::Written
        );
        assert!(fs::symlink_metadata(&dest).unwrap().file_type().is_file());
        write_atomic(&dest, b"new", 0o644).unwrap();
        assert_eq!(fs::read(&victim).unwrap(), b"keep", "victim untouched");
        assert_eq!(fs::read(&dest).unwrap(), b"new");
    }

    #[test]
    fn a_planted_temp_path_fails_the_write_and_is_not_followed() {
        let dir = tempfile::tempdir().unwrap();
        let victim = dir.path().join("victim");
        fs::write(&victim, b"keep").unwrap();
        let dest = dir.path().join("palette.json");
        let planted = dir
            .path()
            .join(format!(".palette.json.tmp.{}", std::process::id()));
        symlink(&victim, &planted).unwrap();

        let err = write_atomic(&dest, b"new", 0o644).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::AlreadyExists);
        assert_eq!(fs::read(&victim).unwrap(), b"keep");
        assert!(!dest.exists());
        assert!(
            fs::symlink_metadata(&planted).is_ok(),
            "a path this call did not create is not removed"
        );
    }

    #[test]
    fn a_failed_rename_leaves_no_temp_file() {
        let dir = tempfile::tempdir().unwrap();
        // rename(file, non-empty directory) fails with EISDIR/ENOTEMPTY.
        let dest = dir.path().join("occupied");
        fs::create_dir(&dest).unwrap();
        fs::write(dest.join("inside"), b"x").unwrap();

        assert!(write_atomic(&dest, b"new", 0o644).is_err());
        assert!(
            leftovers(dir.path()).is_empty(),
            "{:?}",
            leftovers(dir.path())
        );
        assert!(dest.is_dir());
    }

    #[test]
    fn a_missing_directory_is_an_error_with_nothing_created() {
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("absent/palette.json");
        let err = write_atomic(&dest, b"x", 0o644).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::NotFound);
    }
}

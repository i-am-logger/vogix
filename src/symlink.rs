use crate::errors::{Result, VogixError};
use crate::state::State;
use log::debug;
use std::fs;
use std::path::PathBuf;

/// Manages symlinks for theme switching
pub struct SymlinkManager {
    /// Optional override for the themes directory (used for testing)
    themes_dir_override: Option<PathBuf>,
}

impl SymlinkManager {
    pub fn new() -> Self {
        SymlinkManager {
            themes_dir_override: None,
        }
    }

    /// Create a SymlinkManager with a custom themes directory (for testing)
    #[cfg(test)]
    pub fn with_themes_dir(themes_dir: PathBuf) -> Self {
        SymlinkManager {
            themes_dir_override: Some(themes_dir),
        }
    }

    /// Get the themes directory path (~/.local/share/vogix/themes/)
    fn themes_dir(&self) -> PathBuf {
        self.themes_dir_override
            .clone()
            .unwrap_or_else(crate::config::Config::themes_dir)
    }

    /// Update the 'current-theme' symlink to point to the selected theme-variant
    ///
    /// The symlink is at ~/.local/state/vogix/current-theme and points to
    /// ~/.local/share/vogix/themes/{theme}-{variant}
    ///
    /// App configs (e.g., ~/.config/alacritty/alacritty.toml) symlink to
    /// ~/.local/state/vogix/current-theme/{app}/{config} for atomic switching.
    pub fn update_current_symlink(&self, theme: &str, variant: &str) -> Result<()> {
        let themes_dir = self.themes_dir();
        let theme_variant_name = format!("{}-{}", theme, variant);
        let target_path = themes_dir.join(&theme_variant_name);

        // Verify target directory exists (home-manager should have created it)
        if !target_path.exists() {
            return Err(VogixError::ThemeNotFound(format!(
                "Theme-variant directory not found: {}. \
                  This should have been created by the home-manager module.",
                target_path.display()
            )));
        }

        // The current-theme symlink lives in state dir, not themes dir
        let state_dir = State::state_dir()?;
        fs::create_dir_all(&state_dir)?;
        let current_link = state_dir.join("current-theme");

        // Guard: never clobber a non-symlink occupying this path.
        if current_link.exists() && !current_link.is_symlink() {
            return Err(VogixError::symlink(format!(
                "'current-theme' path exists but is not a symlink: {}",
                current_link.display()
            )));
        }

        // Swap the symlink atomically: create it under a unique temp name in the
        // same directory, then rename() over the live path. rename(2) replaces an
        // existing symlink in one step, so `current-theme` is never absent — a
        // remove-then-create would leave a window where every app config that
        // resolves through it dangles (a crash there breaks the whole desktop).
        let tmp_link = state_dir.join(format!("current-theme.tmp.{}", std::process::id()));
        let _ = fs::remove_file(&tmp_link); // clear a stale temp from a crashed run

        #[cfg(unix)]
        std::os::unix::fs::symlink(&target_path, &tmp_link).map_err(|e| {
            VogixError::symlink_with_source("failed to create 'current-theme' symlink", e)
        })?;

        fs::rename(&tmp_link, &current_link).map_err(|e| {
            let _ = fs::remove_file(&tmp_link);
            VogixError::symlink_with_source("failed to swap 'current-theme' symlink", e)
        })?;

        debug!(
            "Updated current-theme: {} -> {}",
            current_link.display(),
            target_path.display()
        );

        Ok(())
    }

    /// Update the 'current-theme' symlink in state directory to point to cached configs
    /// Path: ~/.local/state/vogix/current-theme -> ~/.cache/vogix/themes/{hash}/...
    ///
    /// Currently unused — `maybe_render_templates` no longer touches the state
    /// symlink (see commands/refresh.rs for rationale). Kept as public API
    /// because the cache layout differs from share-themes layout, and any
    /// future "use cache as canonical source" pathway will need this.
    #[allow(dead_code)]
    pub fn update_state_current_symlink(&self, cache_path: &std::path::Path) -> Result<()> {
        let state_dir = State::state_dir()?;

        // Create state directory if it doesn't exist
        fs::create_dir_all(&state_dir)?;

        let current_link = state_dir.join("current-theme");

        // Guard: never clobber a non-symlink occupying this path.
        if current_link.exists() && !current_link.is_symlink() {
            return Err(VogixError::symlink(format!(
                "'current-theme' path exists but is not a symlink: {}",
                current_link.display()
            )));
        }

        // Atomic swap (see update_current_symlink): temp symlink + rename().
        let tmp_link = state_dir.join(format!("current-theme.tmp.{}", std::process::id()));
        let _ = fs::remove_file(&tmp_link);

        #[cfg(unix)]
        std::os::unix::fs::symlink(cache_path, &tmp_link).map_err(|e| {
            VogixError::symlink_with_source("failed to create state 'current-theme' symlink", e)
        })?;

        fs::rename(&tmp_link, &current_link).map_err(|e| {
            let _ = fs::remove_file(&tmp_link);
            VogixError::symlink_with_source("failed to swap state 'current-theme' symlink", e)
        })?;

        debug!(
            "Updated state current-theme: {} -> {}",
            current_link.display(),
            cache_path.display()
        );

        Ok(())
    }
}

impl Default for SymlinkManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_symlink_manager_creation() {
        let _manager = SymlinkManager::new();
        let _default = SymlinkManager::default();
    }

    #[test]
    fn test_themes_dir_default() {
        let manager = SymlinkManager::new();
        // themes_dir uses ~/.local/share/vogix/themes/ via dirs::data_dir()
        let expected = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("/tmp/.local/share"))
            .join("vogix/themes");
        assert_eq!(manager.themes_dir(), expected);
    }

    #[test]
    fn test_themes_dir_override() {
        let custom_path = PathBuf::from("/tmp/test-themes");
        let manager = SymlinkManager::with_themes_dir(custom_path.clone());
        assert_eq!(manager.themes_dir(), custom_path);
    }

    #[test]
    fn test_update_current_symlink_theme_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let themes_dir = temp_dir.path().to_path_buf();

        // Create themes directory but NOT the theme-variant directory
        fs::create_dir_all(&themes_dir).unwrap();

        let manager = SymlinkManager::with_themes_dir(themes_dir);
        let result = manager.update_current_symlink("nonexistent", "theme");

        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("not found"));
    }

    #[test]
    fn test_update_state_current_symlink() {
        let temp_dir = TempDir::new().unwrap();
        let state_dir = temp_dir.path().join("state/vogix");
        let cache_path = temp_dir
            .path()
            .join("cache/themes/hash123/vogix16/yoga/dark");

        // Create cache directory and state directory
        fs::create_dir_all(&cache_path).unwrap();
        fs::create_dir_all(&state_dir).unwrap();

        // Use a manager with custom state dir via direct symlink creation
        // (update_state_current_symlink uses State::state_dir() which we can't override)
        let current_link = state_dir.join("current-theme");

        #[cfg(unix)]
        std::os::unix::fs::symlink(&cache_path, &current_link).unwrap();

        // Verify symlink was created
        assert!(current_link.is_symlink());

        // Verify symlink points to cache path
        let target = fs::read_link(&current_link).unwrap();
        assert_eq!(target, cache_path);
    }

    #[test]
    fn test_update_state_current_symlink_replaces_existing() {
        let temp_dir = TempDir::new().unwrap();
        let state_dir = temp_dir.path().join("state/vogix");
        let cache_path1 = temp_dir.path().join("cache/themes/hash1/vogix16/yoga/dark");
        let cache_path2 = temp_dir
            .path()
            .join("cache/themes/hash2/vogix16/yoga/light");

        fs::create_dir_all(&cache_path1).unwrap();
        fs::create_dir_all(&cache_path2).unwrap();
        fs::create_dir_all(&state_dir).unwrap();

        let current_link = state_dir.join("current-theme");

        // Create first symlink
        #[cfg(unix)]
        std::os::unix::fs::symlink(&cache_path1, &current_link).unwrap();
        let target1 = fs::read_link(&current_link).unwrap();
        assert_eq!(target1, cache_path1);

        // Replace with second symlink
        fs::remove_file(&current_link).unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(&cache_path2, &current_link).unwrap();
        let target2 = fs::read_link(&current_link).unwrap();
        assert_eq!(target2, cache_path2);
    }
}

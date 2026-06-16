//! Template rendering helpers.

use crate::cache::ThemeCache;
use crate::config::Config;
use crate::errors::Result;
use crate::state::State;
use log::debug;
use std::path::PathBuf;

/// Render templates to the on-disk cache if template-based rendering is configured.
///
/// Returns `Ok(Some(path))` when templates rendered, `Ok(None)` when not configured.
///
/// IMPORTANT: this function does NOT update `~/.local/state/vogix/current-theme`.
/// The state symlink is owned exclusively by `SymlinkManager::update_current_symlink`
/// in `execute_side_effects`, which points at `~/.local/share/vogix/themes/{theme}-{variant}`
/// (the home-manager-built tree, the layout app config-symlinks expect). Previously this
/// function ALSO wrote that symlink to the cache path, only to be overwritten by the share
/// path one call later — wasted work, and a footgun if either side ever raced.
pub fn maybe_render_templates(config: &Config, state: &State) -> Result<Option<PathBuf>> {
    if config.templates.is_none() {
        debug!("Template rendering not configured, using pre-generated configs");
        return Ok(None);
    }

    let cache = ThemeCache::from_config(config)?;
    let cache_path = cache.get_or_render(
        &state.current_scheme,
        &state.current_theme,
        &state.current_variant,
    )?;

    debug!("Rendered templates to cache: {}", cache_path.display());
    Ok(Some(cache_path))
}

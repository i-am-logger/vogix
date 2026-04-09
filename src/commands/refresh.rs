//! Template rendering helpers.

use crate::cache::ThemeCache;
use crate::config::Config;
use crate::errors::Result;
use crate::state::State;
use crate::symlink::SymlinkManager;
use log::debug;
use std::path::PathBuf;

/// Render templates to cache and update state symlink if template-based rendering is configured
/// Returns Ok(Some(path)) if templates were rendered, Ok(None) if not configured
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

    let symlink_manager = SymlinkManager::new();
    symlink_manager.update_state_current_symlink(&cache_path)?;

    debug!("Rendered templates to: {}", cache_path.display());
    Ok(Some(cache_path))
}

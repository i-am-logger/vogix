//! Refresh command and template rendering helpers.

use crate::cache::ThemeCache;
use crate::config::Config;
use crate::errors::Result;
use crate::reload::ReloadDispatcher;
use crate::state::State;
use crate::symlink::SymlinkManager;
use crate::theme;
use log::{debug, warn};
use std::path::PathBuf;

/// Handle the `refresh` command - reapply current theme state without changes
pub fn handle_refresh(quiet: bool) -> Result<()> {
    let state = State::load()?;
    let config = Config::load()?;

    theme::verify_theme_variant_exists(&state.current_theme, &state.current_variant)?;

    // Render templates to cache if configured
    if let Some(cache_path) = maybe_render_templates(&config, &state)? {
        debug!(
            "Using template-rendered configs from: {}",
            cache_path.display()
        );
    }

    let symlink_manager = SymlinkManager::new();
    symlink_manager.update_current_symlink(&state.current_theme, &state.current_variant)?;

    let reload_dispatcher = ReloadDispatcher::new();
    let reload_result = reload_dispatcher.reload_apps(&config, quiet);

    // Auto-apply shader if configured
    if let Err(e) = super::shader::maybe_apply_shader(&config, &state) {
        warn!("Shader apply failed: {}", e);
    }

    if reload_result.has_failures() {
        warn!(
            "Refreshed current state ({}/{} reloaded, {} failed)",
            reload_result.success_count,
            reload_result.total_count,
            reload_result.failed_apps.len()
        );
    } else {
        debug!("Refreshed current state");
    }
    Ok(())
}

/// Render templates to cache and update state symlink if template-based rendering is configured
/// Returns Ok(Some(path)) if templates were rendered, Ok(None) if not configured
pub fn maybe_render_templates(config: &Config, state: &State) -> Result<Option<PathBuf>> {
    // Only render if templates are configured
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

    // Update the state directory symlink to point to cached configs
    let symlink_manager = SymlinkManager::new();
    symlink_manager.update_state_current_symlink(&cache_path)?;

    debug!("Rendered templates to: {}", cache_path.display());
    Ok(Some(cache_path))
}

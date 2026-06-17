//! Cache management for rendered theme configurations
//!
//! Manages the cache directory structure:
//! ~/.cache/vogix/themes/{templates-hash}/{scheme}/{theme}/{variant}/
//!   ├── alacritty.toml
//!   ├── btop.conf
//!   └── ...
//!
//! # Module Structure
//! - `paths`: Path resolution (XDG, cache paths, theme sources)
//! - `renderer`: Template rendering to cache

pub mod paths;
mod renderer;
#[cfg(test)]
mod tests;

use crate::config::{Config, TemplatesConfig, ThemeSourcesConfig};
use crate::errors::{Result, VogixError};
use crate::scheme::Scheme;
use log::debug;
use std::fs;
use std::path::PathBuf;

/// Manages the theme configuration cache
pub struct ThemeCache {
    /// Base cache directory (~/.cache/vogix/themes)
    cache_dir: PathBuf,
    /// Templates configuration
    templates: TemplatesConfig,
    /// Theme sources configuration
    theme_sources: ThemeSourcesConfig,
}

impl ThemeCache {
    /// Create a new ThemeCache from config
    pub fn from_config(config: &Config) -> Result<Self> {
        let cache_dir = paths::cache_base_dir()?.join("themes");
        Self::from_config_with_cache_dir(config, cache_dir)
    }

    /// Create a new ThemeCache with a specific cache directory (for testing)
    pub fn from_config_with_cache_dir(config: &Config, cache_dir: PathBuf) -> Result<Self> {
        let templates = config
            .templates
            .clone()
            .ok_or_else(|| VogixError::Config("no templates configuration found".to_string()))?;

        let theme_sources = config.theme_sources.clone().ok_or_else(|| {
            VogixError::Config("no theme_sources configuration found".to_string())
        })?;

        Ok(Self {
            cache_dir,
            templates,
            theme_sources,
        })
    }

    /// Get the cache path for a specific theme variant
    /// Returns: ~/.cache/vogix/themes/{templates-hash}/{scheme}/{theme}/{variant}/
    pub fn variant_cache_path(&self, scheme: &Scheme, theme: &str, variant: &str) -> PathBuf {
        paths::variant_cache_path(
            &self.cache_dir,
            &self.templates.hash,
            scheme,
            theme,
            variant,
        )
    }

    /// Check if a theme variant is cached
    pub fn is_cached(&self, scheme: &Scheme, theme: &str, variant: &str) -> bool {
        let cache_path = self.variant_cache_path(scheme, theme, variant);
        cache_path.exists() && cache_path.is_dir()
    }

    /// Render and cache a theme variant
    /// Returns the cache path where configs were written
    pub fn render_variant(&self, scheme: &Scheme, theme: &str, variant: &str) -> Result<PathBuf> {
        let cache_path = self.variant_cache_path(scheme, theme, variant);

        // Check if already cached
        if self.is_cached(scheme, theme, variant) {
            debug!("Using cached configs for {}/{}/{}", scheme, theme, variant);
            return Ok(cache_path);
        }

        // Render to a .tmp directory then atomically rename into place.
        // Prevents TOCTOU race where two concurrent processes both render.
        let tmp_path = cache_path.with_file_name(format!(
            "{}.tmp",
            cache_path.file_name().unwrap_or_default().to_string_lossy()
        ));
        // Clear any stale .tmp from a previously-crashed render so we never reuse a
        // partial directory (render_to_cache's create_dir_all would no-op over it).
        let _ = std::fs::remove_dir_all(&tmp_path);
        if let Err(e) = renderer::render_to_cache(
            &tmp_path,
            &self.templates.path,
            &self.theme_sources,
            scheme,
            theme,
            variant,
        ) {
            // Don't leave a partial .tmp dir orphaned on failure — clean_stale only
            // scans top-level hash dirs, so it would never collect a variant-level one.
            let _ = std::fs::remove_dir_all(&tmp_path);
            return Err(e);
        }
        // Atomic rename — readers see either the complete set or nothing
        if let Err(e) = std::fs::rename(&tmp_path, &cache_path) {
            // Another process may have raced and won — use their result
            if cache_path.exists() {
                let _ = std::fs::remove_dir_all(&tmp_path);
            } else {
                return Err(e.into());
            }
        }

        Ok(cache_path)
    }

    /// Get or render a theme variant (cache-through)
    pub fn get_or_render(&self, scheme: &Scheme, theme: &str, variant: &str) -> Result<PathBuf> {
        self.render_variant(scheme, theme, variant)
    }

    /// Clean old cache entries (keep only current templates hash)
    ///
    /// Removes cache directories for old template hashes to free disk space.
    /// Called by `vogix cache clean` command.
    pub fn clean_stale(&self) -> Result<usize> {
        let mut removed = 0;

        if !self.cache_dir.exists() {
            return Ok(0);
        }

        for entry in fs::read_dir(&self.cache_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                let dir_name = path.file_name().map(|n| n.to_string_lossy().to_string());

                if let Some(name) = dir_name
                    && name != self.templates.hash
                {
                    debug!("Removing stale cache: {}", path.display());
                    fs::remove_dir_all(&path)?;
                    removed += 1;
                }
            }
        }

        Ok(removed)
    }
}

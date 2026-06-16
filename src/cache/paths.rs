//! Path resolution for theme cache
//!
//! Handles all path-related operations for the cache system:
//! - XDG base directory resolution
//! - Cache path construction
//! - Theme source path resolution

use crate::config::ThemeSourcesConfig;
use crate::errors::{Result, VogixError};
use crate::scheme::Scheme;
use std::path::{Path, PathBuf};

/// Get the base cache directory (~/.cache/vogix)
///
/// Follows XDG Base Directory specification:
/// 1. Uses $XDG_CACHE_HOME/vogix if set
/// 2. Falls back to ~/.cache/vogix
pub fn cache_base_dir() -> Result<PathBuf> {
    if let Ok(xdg_cache) = std::env::var("XDG_CACHE_HOME") {
        return Ok(PathBuf::from(xdg_cache).join("vogix"));
    }

    let home = std::env::var("HOME").map_err(|_| VogixError::Config("HOME not set".to_string()))?;
    Ok(PathBuf::from(home).join(".cache").join("vogix"))
}

/// Build the cache path for a specific theme variant
///
/// Returns: {cache_dir}/{templates_hash}/{scheme}/{theme}/{variant}/
pub fn variant_cache_path(
    cache_dir: &Path,
    templates_hash: &str,
    scheme: &Scheme,
    theme: &str,
    variant: &str,
) -> PathBuf {
    cache_dir
        .join(templates_hash)
        .join(scheme.to_string())
        .join(theme)
        .join(variant)
}

/// Get theme source path for a scheme
pub fn theme_source_path<'a>(
    theme_sources: &'a ThemeSourcesConfig,
    scheme: &Scheme,
) -> &'a PathBuf {
    match scheme {
        Scheme::Vogix16 => &theme_sources.vogix16,
        Scheme::Base16 => &theme_sources.base16,
        Scheme::Base24 => &theme_sources.base24,
        Scheme::Ansi16 => &theme_sources.ansi16,
    }
}

/// Get the path to a theme variant file
///
/// Returns: {source_dir}/{theme}/{variant}.{extension}
/// Extension is .toml for vogix16/ansi16, .yaml for base16/base24
pub fn theme_variant_path(
    theme_sources: &ThemeSourcesConfig,
    scheme: &Scheme,
    theme: &str,
    variant: &str,
) -> PathBuf {
    let source = theme_source_path(theme_sources, scheme);
    let extension = match scheme {
        Scheme::Vogix16 | Scheme::Ansi16 => "toml",
        Scheme::Base16 | Scheme::Base24 => "yaml",
    };
    source
        .join(theme)
        .join(format!("{}.{}", variant, extension))
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: We don't test XDG_CACHE_HOME directly as env::set_var is unsafe
    // and can cause race conditions in parallel tests. The logic is simple
    // enough to trust, and integration tests cover the full path resolution.

    #[test]
    fn test_variant_cache_path_structure() {
        let cache_dir = PathBuf::from("/cache");
        let path = variant_cache_path(&cache_dir, "abc123", &Scheme::Vogix16, "yoga", "night");

        assert_eq!(path, PathBuf::from("/cache/abc123/vogix16/yoga/night"));
    }

    #[test]
    fn test_theme_source_path_vogix16() {
        let sources = ThemeSourcesConfig {
            vogix16: PathBuf::from("/themes/vogix16"),
            base16: PathBuf::from("/themes/base16"),
            base24: PathBuf::from("/themes/base24"),
            ansi16: PathBuf::from("/themes/ansi16"),
        };

        assert_eq!(
            theme_source_path(&sources, &Scheme::Vogix16),
            &PathBuf::from("/themes/vogix16")
        );
    }

    #[test]
    fn test_theme_variant_path_toml_extension() {
        let sources = ThemeSourcesConfig {
            vogix16: PathBuf::from("/themes/vogix16"),
            base16: PathBuf::from("/themes/base16"),
            base24: PathBuf::from("/themes/base24"),
            ansi16: PathBuf::from("/themes/ansi16"),
        };

        let path = theme_variant_path(&sources, &Scheme::Vogix16, "yoga", "night");
        assert_eq!(path, PathBuf::from("/themes/vogix16/yoga/night.toml"));
    }

    #[test]
    fn test_theme_variant_path_yaml_extension() {
        let sources = ThemeSourcesConfig {
            vogix16: PathBuf::from("/themes/vogix16"),
            base16: PathBuf::from("/themes/base16"),
            base24: PathBuf::from("/themes/base24"),
            ansi16: PathBuf::from("/themes/ansi16"),
        };

        let path = theme_variant_path(&sources, &Scheme::Base16, "dracula", "default");
        assert_eq!(path, PathBuf::from("/themes/base16/dracula/default.yaml"));
    }
}

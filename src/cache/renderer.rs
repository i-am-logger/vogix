//! Template rendering for theme cache
//!
//! Handles rendering theme templates to the cache directory.

use crate::config::ThemeSourcesConfig;
use crate::errors::{Result, VogixError};
use crate::scheme::Scheme;
use crate::template;
use crate::theme;
use log::{debug, info};
use std::fs;
use std::path::Path;

use super::paths;

/// Render all templates for a theme variant to the cache directory
///
/// # Arguments
/// * `cache_path` - Target directory for rendered configs
/// * `templates_path` - Base path containing scheme template directories
/// * `theme_sources` - Theme source configuration
/// * `scheme` - Color scheme to use
/// * `theme` - Theme name
/// * `variant` - Variant name
///
/// # Returns
/// The cache path where configs were written
pub fn render_to_cache(
    cache_path: &Path,
    templates_path: &Path,
    theme_sources: &ThemeSourcesConfig,
    scheme: &Scheme,
    theme: &str,
    variant: &str,
) -> Result<()> {
    info!(
        "Rendering configs for {}/{}/{} to cache",
        scheme, theme, variant
    );

    // Create cache directory
    fs::create_dir_all(cache_path)?;

    // Load theme colors from variant file
    let variant_path = paths::theme_variant_path(theme_sources, scheme, theme, variant);
    let colors = theme::load_theme_colors(&variant_path, *scheme)?;

    // The theme identity for `meta.*` template variables (the vogix-desktop
    // theme.json contract embeds it). Polarity is read from the config.toml
    // manifest — the SAME value the Nix render layer wrote there — so the two
    // layers cannot disagree on it; the "dark" fallback mirrors the Nix
    // side's `variantData.polarity or "dark"`.
    let polarity = crate::theme::discover_themes()
        .ok()
        .and_then(|themes| {
            themes
                .into_iter()
                .find(|t| t.name == theme && t.scheme == *scheme)
        })
        .and_then(|t| {
            t.variants
                .into_iter()
                .find(|v| v.name == variant)
                .map(|v| v.polarity)
        })
        .unwrap_or_else(|| "dark".to_string());
    let meta = std::collections::HashMap::from([
        ("theme".to_string(), theme.to_string()),
        ("variant".to_string(), variant.to_string()),
        ("scheme".to_string(), scheme.to_string()),
        ("polarity".to_string(), polarity),
    ]);

    // Find templates for this scheme
    let scheme_templates_path = templates_path.join(scheme.to_string());
    if !scheme_templates_path.exists() {
        return Err(VogixError::Config(format!(
            "templates directory not found: {}",
            scheme_templates_path.display()
        )));
    }

    // Render all .vogix template files
    for entry in fs::read_dir(&scheme_templates_path)? {
        let entry = entry?;
        let template_path = entry.path();

        if template_path.extension().is_some_and(|ext| ext == "vogix") {
            render_template_file(&template_path, cache_path, &colors, &meta)?;
        }
    }

    Ok(())
}

/// Render a single template file to the cache directory
fn render_template_file(
    template_path: &Path,
    cache_path: &Path,
    colors: &std::collections::HashMap<String, String>,
    meta: &std::collections::HashMap<String, String>,
) -> Result<()> {
    // Get output filename (remove .vogix extension)
    let output_name = template_path
        .file_stem()
        .ok_or_else(|| VogixError::Config("invalid template filename".to_string()))?
        .to_string_lossy()
        .to_string();

    let output_path = cache_path.join(&output_name);

    // Render template
    let rendered = template::render_template(template_path, colors, meta)?;

    // Write to cache
    fs::write(&output_path, rendered)?;
    debug!(
        "  Rendered {} -> {}",
        template_path.display(),
        output_path.display()
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::TempDir;

    const TEST_THEME_TOML: &str = include_str!("../../tests/fixtures/test-theme.toml");

    fn setup_test_env(temp_dir: &TempDir) -> (PathBuf, PathBuf, ThemeSourcesConfig) {
        let templates_path = temp_dir.path().join("templates");
        let themes_path = temp_dir.path().join("themes");
        let cache_path = temp_dir.path().join("cache");

        // Create directories
        fs::create_dir_all(templates_path.join("vogix16")).unwrap();
        fs::create_dir_all(themes_path.join("test-theme")).unwrap();
        fs::create_dir_all(&cache_path).unwrap();

        // Create template
        fs::write(
            templates_path.join("vogix16").join("test.toml.vogix"),
            "background = \"{{ colors.background }}\"",
        )
        .unwrap();

        // Create theme file
        fs::write(
            themes_path.join("test-theme").join("dark.toml"),
            TEST_THEME_TOML,
        )
        .unwrap();

        let theme_sources = ThemeSourcesConfig {
            vogix16: themes_path.clone(),
            base16: themes_path.clone(),
            base24: themes_path.clone(),
            ansi16: themes_path,
        };

        (cache_path, templates_path, theme_sources)
    }

    #[test]
    fn test_render_to_cache_creates_output() {
        let temp_dir = TempDir::new().unwrap();
        let (cache_path, templates_path, theme_sources) = setup_test_env(&temp_dir);

        render_to_cache(
            &cache_path,
            &templates_path,
            &theme_sources,
            &Scheme::Vogix16,
            "test-theme",
            "dark",
        )
        .unwrap();

        assert!(cache_path.join("test.toml").exists());
    }

    #[test]
    fn test_render_to_cache_substitutes_colors() {
        let temp_dir = TempDir::new().unwrap();
        let (cache_path, templates_path, theme_sources) = setup_test_env(&temp_dir);

        render_to_cache(
            &cache_path,
            &templates_path,
            &theme_sources,
            &Scheme::Vogix16,
            "test-theme",
            "dark",
        )
        .unwrap();

        let content = fs::read_to_string(cache_path.join("test.toml")).unwrap();
        assert_eq!(content, "background = \"#000000\"");
    }

    #[test]
    fn test_render_to_cache_missing_templates_dir() {
        let temp_dir = TempDir::new().unwrap();
        let cache_path = temp_dir.path().join("cache");
        let templates_path = temp_dir.path().join("nonexistent");
        let theme_sources = ThemeSourcesConfig {
            vogix16: temp_dir.path().to_path_buf(),
            base16: temp_dir.path().to_path_buf(),
            base24: temp_dir.path().to_path_buf(),
            ansi16: temp_dir.path().to_path_buf(),
        };

        let result = render_to_cache(
            &cache_path,
            &templates_path,
            &theme_sources,
            &Scheme::Vogix16,
            "test",
            "dark",
        );

        assert!(result.is_err());
    }
}

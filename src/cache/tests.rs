//! Tests for cache module

use super::*;
use crate::config::{TemplatesConfig, ThemeSourcesConfig};
use std::collections::HashMap;
use std::fs;
use tempfile::TempDir;

// Test fixture: minimal vogix16 theme
const TEST_THEME_TOML: &str = include_str!("../../tests/fixtures/test-theme.toml");

/// Helper to create ThemeCache with temp cache directory for testing
fn create_test_cache(temp_dir: &TempDir, config: &Config) -> ThemeCache {
    let cache_dir = temp_dir.path().join("cache/vogix/themes");
    ThemeCache::from_config_with_cache_dir(config, cache_dir).unwrap()
}

fn create_test_config(temp_dir: &TempDir) -> Config {
    let templates_path = temp_dir.path().join("templates");
    let themes_path = temp_dir.path().join("themes");

    // Create directories
    fs::create_dir_all(templates_path.join("vogix16")).unwrap();
    fs::create_dir_all(themes_path.join("test-theme")).unwrap();

    // Create a simple template
    fs::write(
        templates_path.join("vogix16").join("test.toml.vogix"),
        "background = \"{{ colors.background }}\"",
    )
    .unwrap();

    // Create theme file from fixture
    fs::write(
        themes_path.join("test-theme").join("dark.toml"),
        TEST_THEME_TOML,
    )
    .unwrap();

    Config {
        default_theme: "test-theme".to_string(),
        default_variant: "dark".to_string(),
        apps: HashMap::new(),
        hardware: HashMap::new(),
        templates: Some(TemplatesConfig {
            path: templates_path,
            hash: "test-hash-123".to_string(),
        }),
        theme_sources: Some(ThemeSourcesConfig {
            vogix16: themes_path.clone(),
            base16: themes_path.clone(),
            base24: themes_path.clone(),
            ansi16: themes_path,
        }),
        shader: None,
    }
}

/// Create test config with multiple templates for comprehensive testing
fn create_multi_template_config(temp_dir: &TempDir) -> Config {
    let templates_path = temp_dir.path().join("templates");
    let themes_path = temp_dir.path().join("themes");

    // Create directories
    fs::create_dir_all(templates_path.join("vogix16")).unwrap();
    fs::create_dir_all(themes_path.join("test-theme")).unwrap();

    // Create alacritty template
    fs::write(
        templates_path.join("vogix16").join("alacritty.toml.vogix"),
        r#"[colors.primary]
background = "{{ colors.background }}"
foreground = "{{ colors.foreground_text }}"

[colors.normal]
red = "{{ colors.danger }}"
green = "{{ colors.success }}"
"#,
    )
    .unwrap();

    // Create btop template
    fs::write(
        templates_path.join("vogix16").join("btop.theme.vogix"),
        r#"theme[main_bg]="{{ colors.background }}"
theme[main_fg]="{{ colors.foreground_text }}"
theme[temp_end]="{{ colors.danger }}"
"#,
    )
    .unwrap();

    // Create ripgrep template with hex_to_rgb filter
    fs::write(
        templates_path.join("vogix16").join("ripgrep.conf.vogix"),
        r#"--colors=path:fg:{{ colors.foreground_text | hex_to_rgb }}
--colors=match:fg:{{ colors.active | hex_to_rgb }}
"#,
    )
    .unwrap();

    // Create theme file from fixture
    fs::write(
        themes_path.join("test-theme").join("dark.toml"),
        TEST_THEME_TOML,
    )
    .unwrap();

    Config {
        default_theme: "test-theme".to_string(),
        default_variant: "dark".to_string(),
        apps: HashMap::new(),
        hardware: HashMap::new(),
        templates: Some(TemplatesConfig {
            path: templates_path,
            hash: "multi-hash-456".to_string(),
        }),
        theme_sources: Some(ThemeSourcesConfig {
            vogix16: themes_path.clone(),
            base16: themes_path.clone(),
            base24: themes_path.clone(),
            ansi16: themes_path,
        }),
        shader: None,
    }
}

#[test]
fn test_variant_cache_path() {
    let temp_dir = TempDir::new().unwrap();
    let config = create_test_config(&temp_dir);
    let cache = ThemeCache::from_config(&config).unwrap();

    let path = cache.variant_cache_path(&Scheme::Vogix16, "aikido", "night");

    assert!(path.to_string_lossy().contains("test-hash-123"));
    assert!(path.to_string_lossy().contains("vogix16"));
    assert!(path.to_string_lossy().contains("aikido"));
    assert!(path.to_string_lossy().contains("night"));
}

#[test]
fn test_is_cached_false_when_missing() {
    let temp_dir = TempDir::new().unwrap();
    let config = create_test_config(&temp_dir);
    let cache = ThemeCache::from_config(&config).unwrap();

    assert!(!cache.is_cached(&Scheme::Vogix16, "nonexistent", "dark"));
}

#[test]
fn test_from_config_fails_without_templates() {
    let config = Config::default();
    let result = ThemeCache::from_config(&config);
    assert!(result.is_err());
}

#[test]
fn test_render_variant_creates_cache_directory() {
    let temp_dir = TempDir::new().unwrap();
    let config = create_test_config(&temp_dir);
    let cache = create_test_cache(&temp_dir, &config);

    let cache_path = cache
        .render_variant(&Scheme::Vogix16, "test-theme", "dark")
        .unwrap();

    assert!(cache_path.exists());
    assert!(cache_path.is_dir());
}

#[test]
fn test_render_variant_creates_output_file() {
    let temp_dir = TempDir::new().unwrap();
    let config = create_test_config(&temp_dir);
    let cache = create_test_cache(&temp_dir, &config);

    let cache_path = cache
        .render_variant(&Scheme::Vogix16, "test-theme", "dark")
        .unwrap();

    // Check that test.toml was created (from test.toml.vogix)
    let output_file = cache_path.join("test.toml");
    assert!(
        output_file.exists(),
        "Expected {} to exist",
        output_file.display()
    );
}

#[test]
fn test_render_variant_substitutes_colors() {
    let temp_dir = TempDir::new().unwrap();
    let config = create_test_config(&temp_dir);
    let cache = create_test_cache(&temp_dir, &config);

    let cache_path = cache
        .render_variant(&Scheme::Vogix16, "test-theme", "dark")
        .unwrap();

    let output_file = cache_path.join("test.toml");
    let content = fs::read_to_string(&output_file).unwrap();

    // Test theme has base00 = "#000000", which maps to background
    assert_eq!(content, "background = \"#000000\"");
}

#[test]
fn test_render_variant_multiple_templates() {
    let temp_dir = TempDir::new().unwrap();
    let config = create_multi_template_config(&temp_dir);
    let cache = create_test_cache(&temp_dir, &config);

    let cache_path = cache
        .render_variant(&Scheme::Vogix16, "test-theme", "dark")
        .unwrap();

    // Check all output files exist
    assert!(cache_path.join("alacritty.toml").exists());
    assert!(cache_path.join("btop.theme").exists());
    assert!(cache_path.join("ripgrep.conf").exists());
}

#[test]
fn test_render_variant_alacritty_content() {
    let temp_dir = TempDir::new().unwrap();
    let config = create_multi_template_config(&temp_dir);
    let cache = create_test_cache(&temp_dir, &config);

    let cache_path = cache
        .render_variant(&Scheme::Vogix16, "test-theme", "dark")
        .unwrap();

    let content = fs::read_to_string(cache_path.join("alacritty.toml")).unwrap();

    // Verify color substitution (test-theme.toml has specific colors)
    assert!(content.contains("background = \"#000000\"")); // base00
    assert!(content.contains("foreground = \"#555555\"")); // base05 -> foreground_text
    assert!(content.contains("red = \"#00ff00\"")); // base0B -> danger
    assert!(content.contains("green = \"#ff0000\"")); // base08 -> success
}

#[test]
fn test_render_variant_btop_content() {
    let temp_dir = TempDir::new().unwrap();
    let config = create_multi_template_config(&temp_dir);
    let cache = create_test_cache(&temp_dir, &config);

    let cache_path = cache
        .render_variant(&Scheme::Vogix16, "test-theme", "dark")
        .unwrap();

    let content = fs::read_to_string(cache_path.join("btop.theme")).unwrap();

    assert!(content.contains("theme[main_bg]=\"#000000\""));
    assert!(content.contains("theme[main_fg]=\"#555555\""));
    assert!(content.contains("theme[temp_end]=\"#00ff00\"")); // danger
}

#[test]
fn test_render_variant_ripgrep_with_hex_to_rgb_filter() {
    let temp_dir = TempDir::new().unwrap();
    let config = create_multi_template_config(&temp_dir);
    let cache = create_test_cache(&temp_dir, &config);

    let cache_path = cache
        .render_variant(&Scheme::Vogix16, "test-theme", "dark")
        .unwrap();

    let content = fs::read_to_string(cache_path.join("ripgrep.conf")).unwrap();

    // foreground_text = base05 = "#555555" -> "0x55,0x55,0x55"
    assert!(content.contains("--colors=path:fg:0x55,0x55,0x55"));
    // active = base0C = "#00ffff" -> "0x00,0xff,0xff"
    assert!(content.contains("--colors=match:fg:0x00,0xff,0xff"));
}

#[test]
fn test_is_cached_true_after_render() {
    let temp_dir = TempDir::new().unwrap();
    let config = create_test_config(&temp_dir);
    let cache = create_test_cache(&temp_dir, &config);

    // Initially not cached
    assert!(!cache.is_cached(&Scheme::Vogix16, "test-theme", "dark"));

    // Render
    cache
        .render_variant(&Scheme::Vogix16, "test-theme", "dark")
        .unwrap();

    // Now cached
    assert!(cache.is_cached(&Scheme::Vogix16, "test-theme", "dark"));
}

#[test]
fn test_render_variant_uses_cache() {
    let temp_dir = TempDir::new().unwrap();
    let config = create_test_config(&temp_dir);
    let cache = create_test_cache(&temp_dir, &config);

    // First render
    let path1 = cache
        .render_variant(&Scheme::Vogix16, "test-theme", "dark")
        .unwrap();

    // Modify the cached file to prove cache is used
    let output_file = path1.join("test.toml");
    fs::write(&output_file, "MODIFIED").unwrap();

    // Second render should return same path without re-rendering
    let path2 = cache
        .render_variant(&Scheme::Vogix16, "test-theme", "dark")
        .unwrap();

    assert_eq!(path1, path2);

    // Content should still be modified (proving we didn't re-render)
    let content = fs::read_to_string(&output_file).unwrap();
    assert_eq!(content, "MODIFIED");
}

#[test]
fn test_render_variant_missing_theme() {
    let temp_dir = TempDir::new().unwrap();
    let config = create_test_config(&temp_dir);
    let cache = create_test_cache(&temp_dir, &config);

    let result = cache.render_variant(&Scheme::Vogix16, "nonexistent", "dark");
    assert!(result.is_err());
}

#[test]
fn test_render_variant_missing_variant() {
    let temp_dir = TempDir::new().unwrap();
    let config = create_test_config(&temp_dir);
    let cache = create_test_cache(&temp_dir, &config);

    let result = cache.render_variant(&Scheme::Vogix16, "test-theme", "nonexistent");
    assert!(result.is_err());
}

#[test]
fn test_clean_stale_removes_old_hashes() {
    let temp_dir = TempDir::new().unwrap();
    let config = create_test_config(&temp_dir);
    let cache = create_test_cache(&temp_dir, &config);

    // Create a stale cache directory with old hash
    let stale_dir = temp_dir.path().join("cache/vogix/themes/old-hash-999");
    fs::create_dir_all(&stale_dir).unwrap();
    fs::write(stale_dir.join("test.txt"), "stale content").unwrap();

    // Run clean
    let removed = cache.clean_stale().unwrap();

    assert_eq!(removed, 1);
    assert!(!stale_dir.exists());
}

#[test]
fn test_clean_stale_keeps_current_hash() {
    let temp_dir = TempDir::new().unwrap();
    let config = create_test_config(&temp_dir);
    let cache = create_test_cache(&temp_dir, &config);

    // Render something to create current hash directory
    cache
        .render_variant(&Scheme::Vogix16, "test-theme", "dark")
        .unwrap();

    // Verify current hash exists
    let current_hash_dir = temp_dir.path().join("cache/vogix/themes/test-hash-123");
    assert!(current_hash_dir.exists());

    // Run clean
    let removed = cache.clean_stale().unwrap();

    // Should not remove current hash
    assert_eq!(removed, 0);
    assert!(current_hash_dir.exists());
}

#[test]
fn test_get_or_render_returns_same_as_render_variant() {
    let temp_dir = TempDir::new().unwrap();
    let config = create_test_config(&temp_dir);
    let cache = create_test_cache(&temp_dir, &config);

    let path = cache
        .get_or_render(&Scheme::Vogix16, "test-theme", "dark")
        .unwrap();

    assert!(path.exists());
    assert!(path.join("test.toml").exists());
}

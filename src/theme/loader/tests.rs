//! Tests for theme_loader module

use super::*;
use std::io::Write;
use tempfile::NamedTempFile;

#[test]
fn test_load_theme_file_not_found() {
    let result = load_theme_colors("/nonexistent/path.toml", Scheme::Vogix16);
    assert!(result.is_err());
}

#[test]
fn test_load_vogix16_theme() {
    let mut file = NamedTempFile::new().unwrap();
    write!(
        file,
        r##"polarity = "dark"

[colors]
base00 = "#262626"
base01 = "#333333"
base02 = "#3b3028"
base03 = "#54433a"
base04 = "#6c5d53"
base05 = "#a29990"
base06 = "#cbc3bc"
base07 = "#f6f5f0"
base08 = "#4d5645"
base09 = "#835538"
base0A = "#bfa46f"
base0B = "#d7503c"
base0C = "#8694a8"
base0D = "#658fbd"
base0E = "#896ea4"
base0F = "#7a5c42"
"##
    )
    .unwrap();

    let colors = load_theme_colors(file.path(), Scheme::Vogix16).unwrap();

    // Check base colors
    assert_eq!(colors.get("base00"), Some(&"#262626".to_string()));
    assert_eq!(colors.get("base05"), Some(&"#a29990".to_string()));
    assert_eq!(colors.get("base0B"), Some(&"#d7503c".to_string()));

    // Check semantic mappings
    assert_eq!(colors.get("background"), Some(&"#262626".to_string()));
    assert_eq!(colors.get("foreground_text"), Some(&"#a29990".to_string()));
    assert_eq!(colors.get("danger"), Some(&"#d7503c".to_string()));
    assert_eq!(colors.get("success"), Some(&"#4d5645".to_string()));
}

#[test]
fn test_load_base16_theme() {
    let mut file = NamedTempFile::with_suffix(".yaml").unwrap();
    write!(
        file,
        r##"system: "base16"
name: "Test Theme"
variant: "dark"
palette:
  base00: "#1e1e2e"
  base01: "#181825"
  base02: "#313244"
  base03: "#45475a"
  base04: "#585b70"
  base05: "#cdd6f4"
  base06: "#f5e0dc"
  base07: "#b4befe"
  base08: "#f38ba8"
  base09: "#fab387"
  base0A: "#f9e2af"
  base0B: "#a6e3a1"
  base0C: "#94e2d5"
  base0D: "#89b4fa"
  base0E: "#cba6f7"
  base0F: "#f2cdcd"
"##
    )
    .unwrap();

    let colors = load_theme_colors(file.path(), Scheme::Base16).unwrap();

    assert_eq!(colors.get("base00"), Some(&"#1e1e2e".to_string()));
    assert_eq!(colors.get("base05"), Some(&"#cdd6f4".to_string()));
    assert_eq!(colors.get("base08"), Some(&"#f38ba8".to_string()));
    assert_eq!(colors.get("base0D"), Some(&"#89b4fa".to_string()));
}

#[test]
fn test_load_base24_theme() {
    // Base24 uses the same format as base16
    let mut file = NamedTempFile::with_suffix(".yaml").unwrap();
    write!(
        file,
        r##"system: "base24"
name: "Test Theme"
variant: "dark"
palette:
  base00: "#1e1e2e"
  base01: "#181825"
  base02: "#313244"
  base03: "#45475a"
  base04: "#585b70"
  base05: "#cdd6f4"
  base06: "#f5e0dc"
  base07: "#b4befe"
  base08: "#f38ba8"
  base09: "#fab387"
  base0A: "#f9e2af"
  base0B: "#a6e3a1"
  base0C: "#94e2d5"
  base0D: "#89b4fa"
  base0E: "#cba6f7"
  base0F: "#f2cdcd"
  base10: "#11111b"
  base11: "#1e1e2e"
  base12: "#f38ba8"
  base13: "#f9e2af"
  base14: "#a6e3a1"
  base15: "#94e2d5"
  base16: "#89b4fa"
  base17: "#cba6f7"
"##
    )
    .unwrap();

    let colors = load_theme_colors(file.path(), Scheme::Base24).unwrap();

    // Base24 has all base16 colors plus base10-base17
    assert_eq!(colors.get("base00"), Some(&"#1e1e2e".to_string()));
    assert_eq!(colors.get("base10"), Some(&"#11111b".to_string()));
    assert_eq!(colors.get("base17"), Some(&"#cba6f7".to_string()));
}

#[test]
fn test_load_ansi16_theme() {
    let mut file = NamedTempFile::new().unwrap();
    write!(
        file,
        r##"[colors.primary]
background = "#282a36"
foreground = "#f8f8f2"

[colors.cursor]
cursor = "#f8f8f2"
text = "#282a36"

[colors.selection]
background = "#44475a"
text = "#ffffff"

[colors.normal]
black = "#21222c"
red = "#ff5555"
green = "#50fa7b"
yellow = "#f1fa8c"
blue = "#bd93f9"
magenta = "#ff79c6"
cyan = "#8be9fd"
white = "#f8f8f2"

[colors.bright]
black = "#6272a4"
red = "#ff6e6e"
green = "#69ff94"
yellow = "#ffffa5"
blue = "#d6acff"
magenta = "#ff92df"
cyan = "#a4ffff"
white = "#ffffff"
"##
    )
    .unwrap();

    let colors = load_theme_colors(file.path(), Scheme::Ansi16).unwrap();

    // Primary colors
    assert_eq!(colors.get("background"), Some(&"#282a36".to_string()));
    assert_eq!(colors.get("foreground"), Some(&"#f8f8f2".to_string()));

    // Cursor
    assert_eq!(colors.get("cursor_bg"), Some(&"#f8f8f2".to_string()));
    assert_eq!(colors.get("cursor_fg"), Some(&"#282a36".to_string()));

    // Selection
    assert_eq!(colors.get("selection_bg"), Some(&"#44475a".to_string()));
    assert_eq!(colors.get("selection_fg"), Some(&"#ffffff".to_string()));

    // Normal colors
    assert_eq!(colors.get("color00"), Some(&"#21222c".to_string()));
    assert_eq!(colors.get("color01"), Some(&"#ff5555".to_string()));
    assert_eq!(colors.get("color02"), Some(&"#50fa7b".to_string()));
    assert_eq!(colors.get("color07"), Some(&"#f8f8f2".to_string()));

    // Bright colors
    assert_eq!(colors.get("color08"), Some(&"#6272a4".to_string()));
    assert_eq!(colors.get("color09"), Some(&"#ff6e6e".to_string()));
    assert_eq!(colors.get("color15"), Some(&"#ffffff".to_string()));
}

#[test]
fn test_load_invalid_vogix16_theme() {
    let mut file = NamedTempFile::new().unwrap();
    write!(file, "this is not valid toml {{{{").unwrap();

    let result = load_theme_colors(file.path(), Scheme::Vogix16);
    assert!(result.is_err());
}

#[test]
fn test_load_invalid_base16_theme() {
    let mut file = NamedTempFile::new().unwrap();
    write!(file, "this: is: not: valid: yaml: {{{{").unwrap();

    let result = load_theme_colors(file.path(), Scheme::Base16);
    assert!(result.is_err());
}

#[test]
fn test_load_ansi16_partial_colors() {
    // Test that missing optional sections don't cause errors
    let mut file = NamedTempFile::new().unwrap();
    write!(
        file,
        r##"[colors.primary]
background = "#282a36"
foreground = "#f8f8f2"

[colors.normal]
black = "#21222c"
red = "#ff5555"
"##
    )
    .unwrap();

    let colors = load_theme_colors(file.path(), Scheme::Ansi16).unwrap();

    assert_eq!(colors.get("background"), Some(&"#282a36".to_string()));
    assert_eq!(colors.get("color00"), Some(&"#21222c".to_string()));
    assert_eq!(colors.get("color01"), Some(&"#ff5555".to_string()));
    // Missing colors should not be in the map
    assert!(!colors.contains_key("color02"));
    assert!(!colors.contains_key("cursor_bg"));
}

//! Tests for config module

use super::*;

#[test]
fn test_default_config() {
    let config = Config::default();
    assert_eq!(config.default_theme, "aikido");
    assert_eq!(config.default_variant, "dark");
}

#[test]
fn test_parse_valid_manifest() {
    let manifest = r##"
[default]
theme = "nordic"
variant = "light"

[apps.alacritty]
config_path = "/home/user/.config/alacritty/alacritty.toml"
reload_method = "touch"

[apps.btop]
config_path = "/home/user/.config/btop/btop.conf"
reload_method = "signal"
reload_signal = "USR1"
process_name = "btop"
"##;

    let manifest_value: toml::Value = toml::from_str(manifest).unwrap();

    // Extract values like Config::load() does
    let default_theme = manifest_value
        .get("default")
        .and_then(|d| d.get("theme"))
        .and_then(|t| t.as_str())
        .unwrap_or("aikido");

    let default_variant = manifest_value
        .get("default")
        .and_then(|d| d.get("variant"))
        .and_then(|v| v.as_str())
        .unwrap_or("dark");

    assert_eq!(default_theme, "nordic");
    assert_eq!(default_variant, "light");

    let apps_table = manifest_value
        .get("apps")
        .and_then(|a| a.as_table())
        .unwrap();
    assert!(apps_table.contains_key("alacritty"));
    assert!(apps_table.contains_key("btop"));
}

#[test]
fn test_parse_manifest_with_missing_defaults() {
    let manifest = r##"
[apps.alacritty]
config_path = "/home/user/.config/alacritty/alacritty.toml"
reload_method = "touch"
"##;

    let manifest_value: toml::Value = toml::from_str(manifest).unwrap();

    let default_theme = manifest_value
        .get("default")
        .and_then(|d| d.get("theme"))
        .and_then(|t| t.as_str())
        .unwrap_or("aikido");

    let default_variant = manifest_value
        .get("default")
        .and_then(|d| d.get("variant"))
        .and_then(|v| v.as_str())
        .unwrap_or("dark");

    // Should fall back to defaults
    assert_eq!(default_theme, "aikido");
    assert_eq!(default_variant, "dark");
}

#[test]
fn test_parse_invalid_toml() {
    let invalid_manifest = "this is not valid toml {{{";
    let result: std::result::Result<toml::Value, _> = toml::from_str(invalid_manifest);
    assert!(result.is_err());
}

#[test]
fn test_state_dir_uses_xdg_state() {
    // state_dir() should use XDG state directory (~/.local/state/vogix)
    let result = Config::state_dir();
    let expected = dirs::state_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp/.local/state"))
        .join("vogix");
    assert_eq!(result, expected);
}

#[test]
fn test_data_dir_uses_xdg() {
    // data_dir() should use XDG data directory
    let result = Config::data_dir();
    let expected = dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp/.local/share"))
        .join("vogix");
    assert_eq!(result, expected);
}

#[test]
fn test_themes_dir_under_data_dir() {
    // themes_dir() should be under data_dir()
    let result = Config::themes_dir();
    let expected = Config::data_dir().join("themes");
    assert_eq!(result, expected);
}

#[test]
fn test_parse_templates_config() {
    let manifest = r##"
[default]
theme = "aikido"
variant = "dark"

[templates]
path = "/nix/store/abc123-vogix-templates"
hash = "sha256-abcdef123456"
"##;

    let manifest_value: toml::Value = toml::from_str(manifest).unwrap();
    let templates = Config::parse_templates(&manifest_value);

    assert!(templates.is_some());
    let templates = templates.unwrap();
    assert_eq!(
        templates.path,
        PathBuf::from("/nix/store/abc123-vogix-templates")
    );
    assert_eq!(templates.hash, "sha256-abcdef123456");
}

#[test]
fn test_parse_theme_sources_config() {
    let manifest = r##"
[default]
theme = "aikido"
variant = "dark"

[theme_sources]
vogix16 = "/nix/store/vogix16-themes"
base16 = "/nix/store/tinted-schemes/base16"
base24 = "/nix/store/tinted-schemes/base24"
ansi16 = "/nix/store/iterm2-schemes/ansi16"
"##;

    let manifest_value: toml::Value = toml::from_str(manifest).unwrap();
    let theme_sources = Config::parse_theme_sources(&manifest_value);

    assert!(theme_sources.is_some());
    let theme_sources = theme_sources.unwrap();
    assert_eq!(
        theme_sources.vogix16,
        PathBuf::from("/nix/store/vogix16-themes")
    );
    assert_eq!(
        theme_sources.base16,
        PathBuf::from("/nix/store/tinted-schemes/base16")
    );
}

#[test]
fn test_default_config_has_no_templates() {
    let config = Config::default();
    assert!(config.templates.is_none());
    assert!(config.theme_sources.is_none());
}

#[test]
fn test_parse_apps() {
    let manifest = r##"
[apps.alacritty]
config_path = "/home/user/.config/alacritty/alacritty.toml"
reload_method = "touch"

[apps.btop]
config_path = "/home/user/.config/btop/btop.conf"
reload_method = "signal"
reload_signal = "USR1"
process_name = "btop"

[apps.polybar]
config_path = "/home/user/.config/polybar/config.ini"
reload_method = "command"
reload_command = "polybar-msg cmd restart"
"##;

    let manifest_value: toml::Value = toml::from_str(manifest).unwrap();
    let apps = Config::parse_apps(&manifest_value);

    assert_eq!(apps.len(), 3);

    let alacritty = apps.get("alacritty").unwrap();
    assert_eq!(alacritty.reload_method, "touch");
    assert!(alacritty.reload_signal.is_none());

    let btop = apps.get("btop").unwrap();
    assert_eq!(btop.reload_method, "signal");
    assert_eq!(btop.reload_signal.as_deref(), Some("USR1"));
    assert_eq!(btop.process_name.as_deref(), Some("btop"));

    let polybar = apps.get("polybar").unwrap();
    assert_eq!(polybar.reload_method, "command");
    assert_eq!(
        polybar.reload_command.as_deref(),
        Some("polybar-msg cmd restart")
    );
}

#[test]
fn test_parse_hardware() {
    let manifest = r##"
[hardware."kraken-ring"]
command = "liquidctl --match kraken set external color fixed {{base01}}"

[hardware."keychron-k2-he"]
command = "openrgb -d 'Keychron K2 HE' -m static -c {{base01}}"
"##;

    let manifest_value: toml::Value = toml::from_str(manifest).unwrap();
    let hardware = Config::parse_hardware(&manifest_value);

    assert_eq!(hardware.len(), 2);

    let kraken = hardware.get("kraken-ring").unwrap();
    assert!(kraken.command.contains("liquidctl"));
    assert!(kraken.command.contains("{{base01}}"));

    let keychron = hardware.get("keychron-k2-he").unwrap();
    assert!(keychron.command.contains("openrgb"));
}

#[test]
fn test_parse_hardware_empty() {
    let manifest = r##"
[default]
theme = "aikido"
variant = "dark"
"##;

    let manifest_value: toml::Value = toml::from_str(manifest).unwrap();
    let hardware = Config::parse_hardware(&manifest_value);

    assert!(hardware.is_empty());
}

#[test]
fn test_parse_hardware_missing_command() {
    let manifest = r##"
[hardware."broken-device"]
not_command = "something"
"##;

    let manifest_value: toml::Value = toml::from_str(manifest).unwrap();
    let hardware = Config::parse_hardware(&manifest_value);

    // Device without "command" key should be skipped
    assert!(hardware.is_empty());
}

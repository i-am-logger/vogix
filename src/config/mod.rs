//! Configuration module - system config loading and path resolution.
//!
//! This module provides:
//! - `Config` struct for loading vogix system configuration from /etc/vogix/
//! - `AppMetadata` for application-specific settings
//! - `TemplatesConfig` and `ThemeSourcesConfig` for template-based rendering
//!
//! Path architecture:
//! - `/etc/vogix/config.toml` - System manifest (NixOS module, read-only)
//! - `~/.local/share/vogix/themes/` - Theme packages (home-manager, read-only)
//! - `~/.local/state/vogix/` - Per-user state (CLI managed, mutable)
//! - `~/.cache/vogix/` - Per-user cache (rendered templates)

#[cfg(test)]
mod tests;
mod types;

use crate::errors::{Result, VogixError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

// Re-export types
pub use types::{AppMetadata, HardwareDevice, ShaderConfig, TemplatesConfig, ThemeSourcesConfig};

/// Main configuration loaded from runtime manifest
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub default_theme: String,
    pub default_variant: String,
    pub apps: HashMap<String, AppMetadata>,
    pub hardware: HashMap<String, HardwareDevice>,
    pub templates: Option<TemplatesConfig>,
    pub theme_sources: Option<ThemeSourcesConfig>,
    pub shader: Option<ShaderConfig>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            default_theme: "aikido".to_string(),
            default_variant: "dark".to_string(),
            apps: HashMap::new(),
            hardware: HashMap::new(),
            templates: None,
            theme_sources: None,
            shader: None,
        }
    }
}

impl Config {
    /// Load configuration from the system config (/etc/vogix/config.toml)
    pub fn load() -> Result<Self> {
        let manifest_path = Self::manifest_path()?;

        if !manifest_path.exists() {
            // Return default config if manifest doesn't exist
            return Ok(Config::default());
        }

        let contents = fs::read_to_string(&manifest_path)?;
        let manifest: toml::Value = toml::from_str(&contents).map_err(VogixError::TomlParse)?;

        // Extract config from manifest structure
        let default_theme = manifest
            .get("default")
            .and_then(|d| d.get("theme"))
            .and_then(|t| t.as_str())
            .unwrap_or("aikido")
            .to_string();

        let default_variant = manifest
            .get("default")
            .and_then(|d| d.get("variant"))
            .and_then(|v| v.as_str())
            .unwrap_or("dark")
            .to_string();

        // Parse app metadata from [apps] section
        let apps = Self::parse_apps(&manifest);

        // Parse hardware devices from [hardware] section
        let hardware = Self::parse_hardware(&manifest);

        // Parse templates config
        let templates = Self::parse_templates(&manifest);

        // Parse theme sources config
        let theme_sources = Self::parse_theme_sources(&manifest);

        // Parse shader config
        let shader = Self::parse_shader(&manifest);

        Ok(Config {
            default_theme,
            default_variant,
            apps,
            hardware,
            templates,
            theme_sources,
            shader,
        })
    }

    /// Parse the [apps] section from manifest
    fn parse_apps(manifest: &toml::Value) -> HashMap<String, AppMetadata> {
        manifest
            .get("apps")
            .and_then(|a| a.as_table())
            .map(|apps_table| {
                apps_table
                    .iter()
                    .filter_map(|(app_name, app_data)| {
                        let config_path = app_data.get("config_path")?.as_str()?.to_string();
                        let reload_method = app_data.get("reload_method")?.as_str()?.to_string();
                        let reload_signal = app_data
                            .get("reload_signal")
                            .and_then(|v| v.as_str())
                            .map(String::from);
                        let process_name = app_data
                            .get("process_name")
                            .and_then(|v| v.as_str())
                            .map(String::from);
                        let reload_command = app_data
                            .get("reload_command")
                            .and_then(|v| v.as_str())
                            .map(String::from);
                        let theme_file_path = app_data
                            .get("theme_file_path")
                            .and_then(|v| v.as_str())
                            .map(String::from);

                        Some((
                            app_name.clone(),
                            AppMetadata {
                                config_path,
                                reload_method,
                                reload_signal,
                                process_name,
                                reload_command,
                                theme_file_path,
                            },
                        ))
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Parse the [hardware] section from manifest
    fn parse_hardware(manifest: &toml::Value) -> HashMap<String, HardwareDevice> {
        manifest
            .get("hardware")
            .and_then(|h| h.as_table())
            .map(|hw_table| {
                hw_table
                    .iter()
                    .filter_map(|(device_name, device_data)| {
                        let command = device_data.get("command")?.as_str()?.to_string();
                        Some((device_name.clone(), HardwareDevice { command }))
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Parse the [templates] section from manifest
    fn parse_templates(manifest: &toml::Value) -> Option<TemplatesConfig> {
        manifest
            .get("templates")
            .and_then(|t| t.as_table())
            .and_then(|t| {
                let path = t.get("path")?.as_str()?;
                let hash = t.get("hash")?.as_str()?;
                Some(TemplatesConfig {
                    path: PathBuf::from(path),
                    hash: hash.to_string(),
                })
            })
    }

    /// Parse the [theme_sources] section from manifest
    fn parse_theme_sources(manifest: &toml::Value) -> Option<ThemeSourcesConfig> {
        manifest
            .get("theme_sources")
            .and_then(|t| t.as_table())
            .and_then(|t| {
                let vogix16 = t.get("vogix16")?.as_str()?;
                let base16 = t.get("base16")?.as_str()?;
                let base24 = t.get("base24")?.as_str()?;
                let ansi16 = t.get("ansi16")?.as_str()?;
                Some(ThemeSourcesConfig {
                    vogix16: PathBuf::from(vogix16),
                    base16: PathBuf::from(base16),
                    base24: PathBuf::from(base24),
                    ansi16: PathBuf::from(ansi16),
                })
            })
    }

    /// Parse the [shader] section from manifest
    fn parse_shader(manifest: &toml::Value) -> Option<ShaderConfig> {
        let table = manifest.get("shader")?.as_table()?;
        let enabled = table
            .get("enabled")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let intensity = table
            .get("intensity")
            .and_then(|v| v.as_float())
            .unwrap_or(0.5) as f32;
        let brightness = table
            .get("brightness")
            .and_then(|v| v.as_float())
            .unwrap_or(1.0) as f32;
        let saturation = table
            .get("saturation")
            .and_then(|v| v.as_float())
            .unwrap_or(1.0) as f32;

        Some(ShaderConfig {
            enabled,
            intensity,
            brightness,
            saturation,
        })
    }

    /// Get the config path (~/.local/state/vogix/config.toml)
    fn manifest_path() -> Result<PathBuf> {
        Ok(Self::state_dir().join("config.toml"))
    }

    /// Get the vogix state directory (~/.local/state/vogix/)
    ///
    /// This is where home-manager generates the user configuration.
    /// Contains config.toml with theme definitions, sources, and app reload methods.
    pub fn state_dir() -> PathBuf {
        dirs::state_dir()
            .unwrap_or_else(|| {
                dirs::home_dir()
                    .unwrap_or_else(|| PathBuf::from("/tmp"))
                    .join(".local/state")
            })
            .join("vogix")
    }

    /// Get the vogix data directory (~/.local/share/vogix/)
    ///
    /// This is where home-manager stores per-user theme packages.
    /// Contains themes/ with symlinks to /nix/store packages.
    pub fn data_dir() -> PathBuf {
        dirs::data_dir()
            .unwrap_or_else(|| {
                dirs::home_dir()
                    .unwrap_or_else(|| PathBuf::from("/tmp"))
                    .join(".local/share")
            })
            .join("vogix")
    }

    /// Get the themes directory (~/.local/share/vogix/themes/)
    ///
    /// Theme packages are stored here by home-manager.
    /// Each theme-variant is a symlink to a /nix/store package.
    pub fn themes_dir() -> PathBuf {
        Self::data_dir().join("themes")
    }
}

//! Configuration types for application metadata and template settings.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Configuration for template-based rendering
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TemplatesConfig {
    /// Path to templates directory in /nix/store
    pub path: PathBuf,
    /// Hash of templates for cache invalidation
    pub hash: String,
}

/// Paths to theme source directories for each scheme
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ThemeSourcesConfig {
    pub vogix16: PathBuf,
    pub base16: PathBuf,
    pub base24: PathBuf,
    pub ansi16: PathBuf,
}

/// Configuration for the monochromatic screen shader
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ShaderConfig {
    /// Whether shader is enabled (set by Nix option)
    #[serde(default)]
    pub enabled: bool,
    /// Blend intensity between original and monochrome [0.0..1.0]
    #[serde(default = "default_intensity")]
    pub intensity: f32,
    /// Output brightness multiplier [0.1..2.0]
    #[serde(default = "default_one")]
    pub brightness: f32,
    /// Color saturation adjustment [0.0..2.0]
    #[serde(default = "default_one")]
    pub saturation: f32,
}

fn default_one() -> f32 {
    1.0
}

fn default_intensity() -> f32 {
    0.5
}

impl Default for ShaderConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            intensity: 0.5,
            brightness: 1.0,
            saturation: 1.0,
        }
    }
}

/// Hardware device that receives theme colors on theme change
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HardwareDevice {
    /// Shell command with {{color}} placeholders (e.g. {{active}}, {{base0C}})
    pub command: String,
}

/// Metadata for an application that can be themed
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppMetadata {
    pub config_path: String,
    pub reload_method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reload_signal: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub process_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reload_command: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub theme_file_path: Option<String>,
}

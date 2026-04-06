//! Error types for vogix
//!
//! Uses thiserror for clean, idiomatic error handling with proper error chaining.

use std::io;
use std::path::PathBuf;
use thiserror::Error;

/// All possible errors in vogix
#[derive(Debug, Error)]
pub enum VogixError {
    /// IO operation failed
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    /// Configuration file not found at expected path
    #[error("config file not found: {}", .0.display())]
    ConfigNotFound(PathBuf),

    /// Configuration is invalid or missing required fields
    #[error("config error: {0}")]
    Config(String),

    /// Failed to parse TOML file
    #[error("failed to parse TOML")]
    TomlParse(#[source] toml::de::Error),

    /// Failed to serialize TOML
    #[error("failed to serialize TOML")]
    TomlSerialize(#[source] toml::ser::Error),

    /// Failed to parse YAML file
    #[error("failed to parse YAML")]
    YamlParse(#[source] serde_yaml::Error),

    /// Theme specification is invalid
    #[error("invalid theme: {0}")]
    InvalidTheme(String),

    /// Requested theme does not exist
    #[error("theme not found: {0}")]
    ThemeNotFound(String),

    /// Symlink operation failed
    #[error("symlink error: {message}")]
    Symlink {
        message: String,
        #[source]
        source: Option<io::Error>,
    },

    /// Application reload failed
    #[error("reload error: {message}")]
    Reload {
        message: String,
        #[source]
        source: Option<io::Error>,
    },

    /// Template rendering failed
    #[error("template error: {0}")]
    Template(#[source] tera::Error),

    /// JSON serialization/deserialization failed
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Generic error with message
    #[error("{0}")]
    Generic(String),

    /// hyprctl binary not found in PATH
    #[error("hyprctl not found — is Hyprland installed?")]
    HyprctlNotFound,

    /// Hyprland compositor is not running
    #[error("Hyprland is not running (HYPRLAND_INSTANCE_SIGNATURE not set)")]
    HyprlandNotRunning,

    /// hyprctl command failed
    #[error("hyprctl failed (exit {code}): {detail}")]
    HyprctlFailed { code: i32, detail: String },

    /// Failed to write shader file
    #[error("failed to write shader: {}", path.display())]
    ShaderWrite {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    /// Failed to remove shader file
    #[error("failed to remove shader: {}", path.display())]
    ShaderRemove {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    /// No runtime directory available
    #[error("no runtime directory (XDG_RUNTIME_DIR not set)")]
    NoRuntimeDir,
}

impl From<String> for VogixError {
    fn from(s: String) -> Self {
        Self::Generic(s)
    }
}

// Convenience constructors for structured errors
impl VogixError {
    /// Create a symlink error with just a message
    pub fn symlink(message: impl Into<String>) -> Self {
        Self::Symlink {
            message: message.into(),
            source: None,
        }
    }

    /// Create a symlink error with a source
    pub fn symlink_with_source(message: impl Into<String>, source: io::Error) -> Self {
        Self::Symlink {
            message: message.into(),
            source: Some(source),
        }
    }

    /// Create a reload error with just a message
    pub fn reload(message: impl Into<String>) -> Self {
        Self::Reload {
            message: message.into(),
            source: None,
        }
    }

    /// Create a reload error with a source
    pub fn reload_with_source(message: impl Into<String>, source: io::Error) -> Self {
        Self::Reload {
            message: message.into(),
            source: Some(source),
        }
    }
}

/// Result type alias using VogixError
pub type Result<T> = std::result::Result<T, VogixError>;

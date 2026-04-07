use crate::errors::{Result, VogixError};
use crate::scheme::Scheme;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Theme state data persisted to disk
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct State {
    /// Current color scheme (vogix16, base16, base24, ansi16)
    #[serde(default)]
    pub current_scheme: Scheme,
    /// Current theme name
    pub current_theme: String,
    /// Current variant name (e.g., "dark", "light", "dawn", "moon")
    pub current_variant: String,
    /// Timestamp of last theme application
    pub last_applied: Option<String>,
    /// Whether the shader is enabled (None = follow config, Some = user override)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shader_enabled: Option<bool>,
    /// Shader intensity override [0.0..1.0] (None = use config default)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shader_intensity: Option<f32>,
    /// Shader brightness override [0.1..2.0] (None = use config default)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shader_brightness: Option<f32>,
    /// Shader saturation override [0.0..2.0] (None = use config default)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shader_saturation: Option<f32>,
}

impl Default for State {
    fn default() -> Self {
        State {
            current_scheme: Scheme::default(),
            current_theme: "aikido".to_string(),
            current_variant: "night".to_string(),
            last_applied: None,
            shader_enabled: None,
            shader_intensity: None,
            shader_brightness: None,
            shader_saturation: None,
        }
    }
}

impl State {
    /// Load state from the default state file location
    pub fn load() -> Result<Self> {
        Self::load_from(&Self::default_state_path()?)
    }

    /// Load state from a specific path
    pub fn load_from(state_path: &Path) -> Result<Self> {
        if !state_path.exists() {
            return Ok(State::default());
        }

        let contents = fs::read_to_string(state_path)?;
        let state: State = toml::from_str(&contents).map_err(VogixError::TomlParse)?;

        Ok(state)
    }

    /// Save state to the default state file location
    pub fn save(&self) -> Result<()> {
        self.save_to(&Self::default_state_path()?)
    }

    /// Save state to a specific path
    pub fn save_to(&self, state_path: &Path) -> Result<()> {
        // Create parent directory if it doesn't exist
        if let Some(parent) = state_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut state_to_save = self.clone();
        state_to_save.last_applied = Some(chrono::Utc::now().to_rfc3339());

        let contents = toml::to_string_pretty(&state_to_save).map_err(VogixError::TomlSerialize)?;

        fs::write(state_path, contents)?;
        Ok(())
    }

    /// Get the default state file path
    /// Uses XDG_STATE_HOME (~/.local/state/vogix/state.toml)
    fn default_state_path() -> Result<PathBuf> {
        Ok(Self::state_dir()?.join("state.toml"))
    }

    /// Get the vogix state directory (~/.local/state/vogix/)
    /// This is where persistent state lives (survives reboots, unlike /run)
    pub fn state_dir() -> Result<PathBuf> {
        // Use dirs crate for proper XDG handling (respects XDG_STATE_HOME)
        if let Some(state_home) = dirs::state_dir() {
            return Ok(state_home.join("vogix"));
        }

        // Fallback to ~/.local/state/vogix
        dirs::home_dir()
            .map(|home| home.join(".local").join("state").join("vogix"))
            .ok_or_else(|| VogixError::Config("Could not determine home directory".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_state_creation() {
        let state = State {
            current_scheme: Scheme::Vogix16,
            current_theme: "test".to_string(),
            current_variant: "dark".to_string(),
            ..Default::default()
        };
        assert_eq!(state.current_scheme, Scheme::Vogix16);
        assert_eq!(state.current_theme, "test");
        assert_eq!(state.current_variant, "dark");
    }

    #[test]
    fn test_state_default() {
        let state = State::default();
        assert_eq!(state.current_scheme, Scheme::Vogix16);
        assert_eq!(state.current_theme, "aikido");
        assert_eq!(state.current_variant, "night");
        assert!(state.last_applied.is_none());
    }

    #[test]
    fn test_state_save_and_load() {
        let temp_dir = TempDir::new().unwrap();
        let state_path = temp_dir.path().join("state.toml");

        let state = State {
            current_scheme: Scheme::Base16,
            current_theme: "rose-pine".to_string(),
            current_variant: "moon".to_string(),
            ..Default::default()
        };

        // Save state to temp path
        state.save_to(&state_path).unwrap();

        // Load state back from temp path
        let loaded = State::load_from(&state_path).unwrap();

        assert_eq!(loaded.current_scheme, Scheme::Base16);
        assert_eq!(loaded.current_theme, "rose-pine");
        assert_eq!(loaded.current_variant, "moon");
        assert!(loaded.last_applied.is_some()); // save_to() sets timestamp
    }

    #[test]
    fn test_state_load_missing_returns_default() {
        let temp_dir = TempDir::new().unwrap();
        let nonexistent_path = temp_dir.path().join("nonexistent/state.toml");

        // Load from non-existent path - should return default
        let loaded = State::load_from(&nonexistent_path).unwrap();

        assert_eq!(loaded.current_scheme, Scheme::Vogix16);
        assert_eq!(loaded.current_theme, "aikido");
        assert_eq!(loaded.current_variant, "night");
    }

    #[test]
    fn test_state_dir_returns_vogix_subdirectory() {
        // state_dir() should return a path ending in "vogix"
        let state_dir = State::state_dir().unwrap();
        assert!(state_dir.ends_with("vogix"));
    }

    #[test]
    fn test_state_serialization_format() {
        let state = State {
            current_scheme: Scheme::Ansi16,
            current_theme: "dracula".to_string(),
            current_variant: "default".to_string(),
            last_applied: Some("2024-01-01T00:00:00Z".to_string()),
            ..Default::default()
        };

        let serialized = toml::to_string_pretty(&state).unwrap();

        assert!(serialized.contains("current_scheme = \"ansi16\""));
        assert!(serialized.contains("current_theme = \"dracula\""));
        assert!(serialized.contains("current_variant = \"default\""));
    }

    #[test]
    fn test_state_backward_compat_ignores_unknown_fields() {
        // State files with extra fields should still load fine
        let temp_dir = TempDir::new().unwrap();
        let state_path = temp_dir.path().join("state.toml");
        fs::write(
            &state_path,
            r#"
current_scheme = "vogix16"
current_theme = "aikido"
current_variant = "night"
shader_enabled = true
"#,
        )
        .unwrap();

        let loaded = State::load_from(&state_path).unwrap();
        assert_eq!(loaded.current_theme, "aikido");
    }
}

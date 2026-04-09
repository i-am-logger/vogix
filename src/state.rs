use crate::errors::{Result, VogixError};
use crate::scheme::Scheme;
use praxis::engine::Situation;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Shader state — On with params, Off, or Auto (follow config default)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "lowercase")]
pub enum ShaderState {
    Off,
    On {
        #[serde(default = "default_intensity")]
        intensity: f32,
        #[serde(default = "default_one")]
        brightness: f32,
        #[serde(default = "default_one")]
        saturation: f32,
    },
    /// Follow config default (user hasn't explicitly toggled)
    Auto,
}

fn default_intensity() -> f32 {
    0.5
}
fn default_one() -> f32 {
    1.0
}

impl Default for ShaderState {
    fn default() -> Self {
        ShaderState::Auto
    }
}

impl ShaderState {
    pub fn is_on(&self) -> bool {
        matches!(self, ShaderState::On { .. })
    }

    pub fn params(&self) -> Option<(f32, f32, f32)> {
        match self {
            ShaderState::On {
                intensity,
                brightness,
                saturation,
            } => Some((*intensity, *brightness, *saturation)),
            _ => None,
        }
    }
}

/// Vogix state — implements praxis Situation for engine-driven state management
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
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
    /// Shader state (On/Off/Auto)
    #[serde(default)]
    pub shader: ShaderState,
}

impl Situation for State {
    fn describe(&self) -> String {
        let shader_desc = match &self.shader {
            ShaderState::Off => "off".to_string(),
            ShaderState::On { intensity, .. } => format!("on(i={:.2})", intensity),
            ShaderState::Auto => "auto".to_string(),
        };
        format!(
            "{}/{}/{} shader={}",
            self.current_scheme, self.current_theme, self.current_variant, shader_desc
        )
    }

    fn is_terminal(&self) -> bool {
        false
    }
}

impl Default for State {
    fn default() -> Self {
        State {
            current_scheme: Scheme::default(),
            current_theme: "aikido".to_string(),
            current_variant: "night".to_string(),
            last_applied: None,
            shader: ShaderState::Auto,
        }
    }
}

impl State {
    /// Load state from the default state file location
    pub fn load() -> Result<Self> {
        Self::load_from(&Self::default_state_path()?)
    }

    /// Load state from a specific path, with migration from old format
    pub fn load_from(state_path: &Path) -> Result<Self> {
        if !state_path.exists() {
            return Ok(State::default());
        }

        let contents = fs::read_to_string(state_path)?;

        // Detect format: old format has flat shader_enabled/shader_intensity fields
        if contents.contains("shader_enabled") || contents.contains("shader_intensity") {
            return Self::migrate_old_format(&contents);
        }

        // New format (ShaderState enum with [shader] section)
        let state: State = toml::from_str(&contents).map_err(VogixError::TomlParse)?;
        Ok(state)
    }

    /// Migrate from old state.toml format (flat shader_enabled/intensity/brightness/saturation)
    fn migrate_old_format(contents: &str) -> Result<Self> {
        #[derive(Deserialize)]
        struct OldState {
            #[serde(default)]
            current_scheme: Scheme,
            current_theme: String,
            current_variant: String,
            last_applied: Option<String>,
            #[serde(default)]
            shader_enabled: Option<bool>,
            #[serde(default)]
            shader_intensity: Option<f32>,
            #[serde(default)]
            shader_brightness: Option<f32>,
            #[serde(default)]
            shader_saturation: Option<f32>,
        }

        let old: OldState = toml::from_str(contents).map_err(VogixError::TomlParse)?;

        let shader = match old.shader_enabled {
            Some(true) => ShaderState::On {
                intensity: old.shader_intensity.unwrap_or(0.5),
                brightness: old.shader_brightness.unwrap_or(1.0),
                saturation: old.shader_saturation.unwrap_or(1.0),
            },
            Some(false) => ShaderState::Off,
            None => ShaderState::Auto,
        };

        Ok(State {
            current_scheme: old.current_scheme,
            current_theme: old.current_theme,
            current_variant: old.current_variant,
            last_applied: old.last_applied,
            shader,
        })
    }

    /// Save state to the default state file location
    pub fn save(&self) -> Result<()> {
        self.save_to(&Self::default_state_path()?)
    }

    /// Save state to a specific path
    pub fn save_to(&self, state_path: &Path) -> Result<()> {
        if let Some(parent) = state_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut state_to_save = self.clone();
        state_to_save.last_applied = Some(chrono::Utc::now().to_rfc3339());

        let contents =
            toml::to_string_pretty(&state_to_save).map_err(VogixError::TomlSerialize)?;

        fs::write(state_path, contents)?;
        Ok(())
    }

    fn default_state_path() -> Result<PathBuf> {
        Ok(Self::state_dir()?.join("state.toml"))
    }

    pub fn state_dir() -> Result<PathBuf> {
        if let Some(state_home) = dirs::state_dir() {
            return Ok(state_home.join("vogix"));
        }

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
    fn test_state_default() {
        let state = State::default();
        assert_eq!(state.current_scheme, Scheme::Vogix16);
        assert_eq!(state.current_theme, "aikido");
        assert_eq!(state.current_variant, "night");
        assert!(state.last_applied.is_none());
        assert_eq!(state.shader, ShaderState::Auto);
    }

    #[test]
    fn test_shader_state_enum() {
        let on = ShaderState::On {
            intensity: 0.5,
            brightness: 1.0,
            saturation: 1.0,
        };
        assert!(on.is_on());
        assert_eq!(on.params(), Some((0.5, 1.0, 1.0)));

        assert!(!ShaderState::Off.is_on());
        assert_eq!(ShaderState::Off.params(), None);

        assert!(!ShaderState::Auto.is_on());
        assert_eq!(ShaderState::Auto.params(), None);
    }

    #[test]
    fn test_state_save_and_load() {
        let temp_dir = TempDir::new().unwrap();
        let state_path = temp_dir.path().join("state.toml");

        let state = State {
            current_scheme: Scheme::Base16,
            current_theme: "rose-pine".to_string(),
            current_variant: "moon".to_string(),
            shader: ShaderState::On {
                intensity: 0.3,
                brightness: 1.2,
                saturation: 0.8,
            },
            ..Default::default()
        };

        state.save_to(&state_path).unwrap();
        let loaded = State::load_from(&state_path).unwrap();

        assert_eq!(loaded.current_scheme, Scheme::Base16);
        assert_eq!(loaded.current_theme, "rose-pine");
        assert_eq!(loaded.current_variant, "moon");
        assert!(loaded.shader.is_on());
        assert_eq!(loaded.shader.params(), Some((0.3, 1.2, 0.8)));
    }

    #[test]
    fn test_migrate_old_format() {
        let temp_dir = TempDir::new().unwrap();
        let state_path = temp_dir.path().join("state.toml");
        fs::write(
            &state_path,
            r#"
current_scheme = "vogix16"
current_theme = "aikido"
current_variant = "night"
shader_enabled = true
shader_intensity = 0.4
"#,
        )
        .unwrap();

        let loaded = State::load_from(&state_path).unwrap();
        assert_eq!(loaded.current_theme, "aikido");
        assert!(loaded.shader.is_on());
        assert_eq!(loaded.shader.params(), Some((0.4, 1.0, 1.0)));
    }

    #[test]
    fn test_migrate_old_format_disabled() {
        let temp_dir = TempDir::new().unwrap();
        let state_path = temp_dir.path().join("state.toml");
        fs::write(
            &state_path,
            r#"
current_scheme = "vogix16"
current_theme = "aikido"
current_variant = "night"
shader_enabled = false
"#,
        )
        .unwrap();

        let loaded = State::load_from(&state_path).unwrap();
        assert_eq!(loaded.shader, ShaderState::Off);
    }

    #[test]
    fn test_migrate_old_format_auto() {
        let temp_dir = TempDir::new().unwrap();
        let state_path = temp_dir.path().join("state.toml");
        fs::write(
            &state_path,
            r#"
current_scheme = "vogix16"
current_theme = "aikido"
current_variant = "night"
"#,
        )
        .unwrap();

        let loaded = State::load_from(&state_path).unwrap();
        assert_eq!(loaded.shader, ShaderState::Auto);
    }

    #[test]
    fn test_situation_describe() {
        let state = State {
            current_scheme: Scheme::Vogix16,
            current_theme: "aikido".to_string(),
            current_variant: "night".to_string(),
            shader: ShaderState::On {
                intensity: 0.5,
                brightness: 1.0,
                saturation: 1.0,
            },
            ..Default::default()
        };
        assert!(state.describe().contains("aikido"));
        assert!(state.describe().contains("on(i=0.50)"));
    }

    #[test]
    fn test_situation_not_terminal() {
        let state = State::default();
        assert!(!state.is_terminal());
    }

    #[test]
    fn test_state_load_missing_returns_default() {
        let temp_dir = TempDir::new().unwrap();
        let nonexistent_path = temp_dir.path().join("nonexistent/state.toml");
        let loaded = State::load_from(&nonexistent_path).unwrap();
        assert_eq!(loaded.current_theme, "aikido");
    }

    #[test]
    fn test_state_dir_returns_vogix_subdirectory() {
        let state_dir = State::state_dir().unwrap();
        assert!(state_dir.ends_with("vogix"));
    }
}

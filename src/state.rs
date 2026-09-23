use crate::config::Config;
use crate::errors::{Result, VogixError};
use crate::scheme::Scheme;
use pr4xis::engine::Situation;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Shader state — On with params, Off, or Auto (follow config default)
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
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
    #[default]
    Auto,
}

fn default_intensity() -> f32 {
    0.5
}
fn default_one() -> f32 {
    1.0
}

impl ShaderState {
    pub fn is_on(&self) -> bool {
        matches!(self, ShaderState::On { .. })
    }

    #[cfg(test)]
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
    /// Current interaction mode
    #[serde(default = "default_mode")]
    pub current_mode: String,
}

fn default_mode() -> String {
    "app".to_string()
}

impl Situation for State {}

impl State {
    pub fn describe(&self) -> String {
        let shader_desc = match &self.shader {
            ShaderState::Off => "off".to_string(),
            ShaderState::On { intensity, .. } => format!("on(i={:.2})", intensity),
            ShaderState::Auto => "auto".to_string(),
        };
        format!(
            "{}/{}/{} shader={} mode={}",
            self.current_scheme,
            self.current_theme,
            self.current_variant,
            shader_desc,
            self.current_mode
        )
    }

    #[cfg(test)]
    pub fn is_terminal(&self) -> bool {
        false
    }
}

/// A fixed state for tests; a user's first state is `State::initial`.
#[cfg(test)]
impl Default for State {
    fn default() -> Self {
        State {
            current_scheme: Scheme::default(),
            current_theme: "yoga".to_string(),
            current_variant: "night".to_string(),
            last_applied: None,
            shader: ShaderState::Auto,
            current_mode: "app".to_string(),
        }
    }
}

impl State {
    /// The state of a user who has no state file: the configured default
    /// theme (`[default]` in config.toml) in its scheme, the shader following
    /// the config, the base mode.
    pub fn initial(config: &Config) -> Self {
        State {
            current_scheme: config.default_scheme,
            current_theme: config.default_theme.clone(),
            current_variant: config.default_variant.clone(),
            last_applied: None,
            shader: ShaderState::Auto,
            current_mode: default_mode(),
        }
    }

    /// Load state from the default state file location; without one, the
    /// config's initial state.
    pub fn load(config: &Config) -> Result<Self> {
        Self::load_from(&Self::default_state_path()?, config)
    }

    /// The user's config, and their state loaded against it.
    pub fn load_with_config() -> Result<(Config, Self)> {
        let config = Config::load()?;
        let state = Self::load(&config)?;
        Ok((config, state))
    }

    /// Load state from a specific path, with migration from old format;
    /// without a file there, the config's initial state.
    pub fn load_from(state_path: &Path, config: &Config) -> Result<Self> {
        if !state_path.exists() {
            return Ok(State::initial(config));
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
            current_mode: "app".to_string(),
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

        let contents = toml::to_string_pretty(&state_to_save).map_err(VogixError::TomlSerialize)?;

        // Atomic write: write to .tmp then rename — prevents corruption on concurrent invocations
        let tmp = state_path.with_extension("toml.tmp");
        fs::write(&tmp, contents)?;
        fs::rename(&tmp, state_path)?;
        Ok(())
    }

    /// Update just `current_mode` in the persisted state, without bumping `last_applied`.
    ///
    /// Used by the daemon when a Hyprland submap event lands — the user didn't
    /// re-apply a theme, they switched mode. Keeping `last_applied` truthful
    /// matters because it's user-visible in `vogix theme status` and used as
    /// a freshness signal elsewhere.
    ///
    /// Concurrency: theme-changing CLIs use `save_to` (full write); a race here
    /// is last-write-wins on the whole file. Atomic via tmp+rename, so no
    /// torn writes. The brief window where a mode update could clobber a theme
    /// update is acceptable — both end states are valid; only `last_applied`
    /// might briefly read stale.
    ///
    /// Without a state file, the mode is written onto the config's initial
    /// state.
    pub fn save_current_mode(config: &Config, mode: &str) -> Result<()> {
        Self::save_current_mode_to(&Self::default_state_path()?, config, mode)
    }

    fn save_current_mode_to(path: &Path, config: &Config, mode: &str) -> Result<()> {
        let mut state = Self::load_from(path, config)?;

        if state.current_mode == mode {
            return Ok(()); // no-op, avoid pointless write
        }

        state.current_mode = mode.to_string();

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let contents = toml::to_string_pretty(&state).map_err(VogixError::TomlSerialize)?;
        let tmp = path.with_extension("toml.tmp");
        fs::write(&tmp, contents)?;
        fs::rename(&tmp, path)?;
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
        assert_eq!(state.current_theme, "yoga");
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
        let loaded = State::load_from(&state_path, &Config::default()).unwrap();

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
current_theme = "yoga"
current_variant = "night"
shader_enabled = true
shader_intensity = 0.4
"#,
        )
        .unwrap();

        let loaded = State::load_from(&state_path, &Config::default()).unwrap();
        assert_eq!(loaded.current_theme, "yoga");
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
current_theme = "yoga"
current_variant = "night"
shader_enabled = false
"#,
        )
        .unwrap();

        let loaded = State::load_from(&state_path, &Config::default()).unwrap();
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
current_theme = "yoga"
current_variant = "night"
"#,
        )
        .unwrap();

        let loaded = State::load_from(&state_path, &Config::default()).unwrap();
        assert_eq!(loaded.shader, ShaderState::Auto);
    }

    #[test]
    fn test_situation_describe() {
        let state = State {
            current_scheme: Scheme::Vogix16,
            current_theme: "yoga".to_string(),
            current_variant: "night".to_string(),
            shader: ShaderState::On {
                intensity: 0.5,
                brightness: 1.0,
                saturation: 1.0,
            },
            ..Default::default()
        };
        assert!(state.describe().contains("yoga"));
        assert!(state.describe().contains("on(i=0.50)"));
    }

    #[test]
    fn test_situation_not_terminal() {
        let state = State::default();
        assert!(!state.is_terminal());
    }

    /// A config.toml as home-manager renders it, its default theme `theme`
    /// at `variant`, declaring a vogix16 `desert` and a base16 `dracula`.
    fn rendered_config(theme: &str, variant: &str) -> Config {
        Config::from_manifest(&format!(
            r#"
[default]
theme = "{theme}"
variant = "{variant}"

[themes."desert"]
scheme = "vogix16"
variants = ["day", "night"]
day = {{ polarity = "light", order = 0 }}
night = {{ polarity = "dark", order = 1 }}

[themes."dracula"]
scheme = "base16"
variants = ["dracula"]
dracula = {{ polarity = "dark", order = 0 }}
"#
        ))
        .unwrap()
    }

    #[test]
    fn a_missing_state_file_starts_at_the_configured_default_theme() {
        let temp_dir = TempDir::new().unwrap();
        let nonexistent_path = temp_dir.path().join("nonexistent/state.toml");
        let loaded =
            State::load_from(&nonexistent_path, &rendered_config("desert", "day")).unwrap();
        assert_eq!(
            loaded,
            State {
                current_scheme: Scheme::Vogix16,
                current_theme: "desert".to_string(),
                current_variant: "day".to_string(),
                last_applied: None,
                shader: ShaderState::Auto,
                current_mode: "app".to_string(),
            }
        );
    }

    #[test]
    fn the_first_state_takes_the_default_theme_s_scheme() {
        let temp_dir = TempDir::new().unwrap();
        let loaded = State::load_from(
            &temp_dir.path().join("state.toml"),
            &rendered_config("dracula", "dracula"),
        )
        .unwrap();
        assert_eq!(loaded.current_scheme, Scheme::Base16);
        assert_eq!(loaded.current_theme, "dracula");
        assert_eq!(loaded.current_variant, "dracula");
    }

    #[test]
    fn a_state_file_wins_over_the_configured_default_theme() {
        let temp_dir = TempDir::new().unwrap();
        let state_path = temp_dir.path().join("state.toml");
        State::default().save_to(&state_path).unwrap();
        let loaded = State::load_from(&state_path, &rendered_config("desert", "day")).unwrap();
        assert_eq!(loaded.current_theme, "yoga");
        assert_eq!(loaded.current_variant, "night");
    }

    #[test]
    fn a_mode_saved_without_a_state_file_lands_on_the_configured_default_theme() {
        let temp_dir = TempDir::new().unwrap();
        let state_path = temp_dir.path().join("state.toml");
        State::save_current_mode_to(&state_path, &rendered_config("desert", "night"), "normal")
            .unwrap();
        // Read back under a different default, so only the file answers.
        let saved = State::load_from(&state_path, &Config::default()).unwrap();
        assert_eq!(saved.current_theme, "desert");
        assert_eq!(saved.current_variant, "night");
        assert_eq!(saved.current_mode, "normal");
    }

    #[test]
    fn test_state_dir_returns_vogix_subdirectory() {
        let state_dir = State::state_dir().unwrap();
        assert!(state_dir.ends_with("vogix"));
    }
}

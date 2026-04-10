//! Theme state history — undo/redo across CLI invocations.
//!
//! Persists a stack of VogixState snapshots to disk so that
//! `vogix theme undo` / `vogix theme redo` work across reboots.
//! Separate from session undo (which restores window layouts).

use crate::errors::{Result, VogixError};
use crate::state::State;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

const MAX_HISTORY: usize = 20;

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct History {
    pub past: Vec<State>,
    pub future: Vec<State>,
}

impl History {
    pub fn load() -> Result<Self> {
        let path = Self::path()?;
        if !path.exists() {
            return Ok(History::default());
        }
        let contents = fs::read_to_string(&path)?;
        serde_json::from_str(&contents).map_err(|e| VogixError::Config(e.to_string()))
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::path()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let contents =
            serde_json::to_string_pretty(self).map_err(|e| VogixError::Config(e.to_string()))?;
        fs::write(&path, contents)?;
        Ok(())
    }

    /// Push current state before a transition. Clears redo stack.
    pub fn push(&mut self, state: &State) {
        self.past.push(state.clone());
        if self.past.len() > MAX_HISTORY {
            self.past.remove(0);
        }
        self.future.clear();
    }

    /// Pop from past (undo). Returns the previous state.
    pub fn undo(&mut self, current: &State) -> Option<State> {
        let prev = self.past.pop()?;
        self.future.push(current.clone());
        Some(prev)
    }

    /// Pop from future (redo). Returns the next state.
    pub fn redo(&mut self, current: &State) -> Option<State> {
        let next = self.future.pop()?;
        self.past.push(current.clone());
        Some(next)
    }

    #[cfg(test)]
    pub fn undo_depth(&self) -> usize {
        self.past.len()
    }

    #[cfg(test)]
    pub fn redo_depth(&self) -> usize {
        self.future.len()
    }

    fn path() -> Result<PathBuf> {
        Ok(State::state_dir()?.join("history.json"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scheme::Scheme;
    use crate::state::ShaderState;

    fn state(theme: &str) -> State {
        State {
            current_theme: theme.to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn test_push_and_undo() {
        let mut history = History::default();

        history.push(&state("aikido"));
        history.push(&state("catppuccin"));

        assert_eq!(history.undo_depth(), 2);

        let prev = history.undo(&state("gruvbox")).unwrap();
        assert_eq!(prev.current_theme, "catppuccin");

        let prev = history.undo(&state("catppuccin")).unwrap();
        assert_eq!(prev.current_theme, "aikido");

        assert!(history.undo(&state("aikido")).is_none());
    }

    #[test]
    fn test_redo() {
        let mut history = History::default();
        history.push(&state("aikido"));

        let prev = history.undo(&state("catppuccin")).unwrap();
        assert_eq!(prev.current_theme, "aikido");

        let next = history.redo(&state("aikido")).unwrap();
        assert_eq!(next.current_theme, "catppuccin");

        assert!(history.redo(&state("catppuccin")).is_none());
    }

    #[test]
    fn test_push_clears_future() {
        let mut history = History::default();
        history.push(&state("aikido"));

        history.undo(&state("catppuccin"));
        assert_eq!(history.redo_depth(), 1);

        // New push clears redo
        history.push(&state("gruvbox"));
        assert_eq!(history.redo_depth(), 0);
    }

    #[test]
    fn test_max_history() {
        let mut history = History::default();
        for i in 0..25 {
            history.push(&state(&format!("theme-{}", i)));
        }
        assert_eq!(history.undo_depth(), MAX_HISTORY);
    }

    #[test]
    fn test_save_and_load() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let path = temp_dir.path().join("history.json");

        let mut history = History::default();
        history.push(&state("aikido"));
        history.push(&state("catppuccin"));

        let contents = serde_json::to_string_pretty(&history).unwrap();
        fs::write(&path, contents).unwrap();

        let loaded: History = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(loaded.undo_depth(), 2);
    }

    #[test]
    fn test_shader_state_serializes_in_history() {
        let mut history = History::default();
        let s = State {
            current_theme: "aikido".to_string(),
            shader: ShaderState::On {
                intensity: 0.3,
                brightness: 1.0,
                saturation: 1.0,
            },
            ..Default::default()
        };
        history.push(&s);

        let json = serde_json::to_string(&history).unwrap();
        let loaded: History = serde_json::from_str(&json).unwrap();
        assert!(loaded.past[0].shader.is_on());
    }
}

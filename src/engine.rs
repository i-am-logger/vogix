//! Praxis engine integration for vogix.
//!
//! Defines VogixAction (all state-mutating operations) and preconditions.
//! The apply function is pure — side effects happen after engine commits.

use crate::scheme::Scheme;
use crate::state::{ShaderState, State};
use pr4xis::engine::{Action, Engine, Precondition};
use pr4xis::logic::proof::{Counterexample, SimpleCounterexample, SimpleProof, Verdict};
use pr4xis::ontology::meta::{Citation, Label, ModulePath, OntologyName, Provenance};

/// Build a [`Provenance`] for a precondition proof/counterexample, preserving
/// the rule name and the human-readable reason text.
fn meta(name: &'static str, description: &str) -> Provenance {
    Provenance {
        name: OntologyName::new_static(name),
        description: Label::new(description.to_string()),
        citation: Citation::EMPTY,
        module_path: ModulePath::new_static(module_path!()),
    }
}

/// All state-mutating operations in vogix
#[derive(Debug, Clone)]
pub enum VogixAction {
    SetTheme {
        scheme: Option<Scheme>,
        theme: Option<String>,
        variant: Option<String>,
    },
    ShaderOn {
        intensity: Option<f32>,
        brightness: Option<f32>,
        saturation: Option<f32>,
    },
    ShaderOff,
    ShaderToggle,
    ShaderParam {
        param: ShaderParam,
        value: f32,
    },
    ModeChange {
        target: String,
    },
    Refresh,
}

#[derive(Debug, Clone)]
pub enum ShaderParam {
    Intensity,
    Brightness,
    Saturation,
}

impl Action for VogixAction {
    type Sit = State;
}

impl VogixAction {
    pub fn describe(&self) -> String {
        match self {
            VogixAction::SetTheme {
                scheme,
                theme,
                variant,
            } => format!(
                "set theme (scheme={:?}, theme={:?}, variant={:?})",
                scheme, theme, variant
            ),
            VogixAction::ShaderOn { .. } => "shader on".into(),
            VogixAction::ShaderOff => "shader off".into(),
            VogixAction::ShaderToggle => "shader toggle".into(),
            VogixAction::ShaderParam { param, value } => {
                format!("shader {:?} = {:.2}", param, value)
            }
            VogixAction::ModeChange { target } => format!("mode → {}", target),
            VogixAction::Refresh => "refresh".into(),
        }
    }
}

/// Pure state transition — no I/O, no side effects
pub fn apply_action(state: &State, action: &VogixAction) -> Result<State, Box<dyn Counterexample>> {
    let mut next = state.clone();

    match action {
        VogixAction::SetTheme {
            scheme,
            theme,
            variant,
        } => {
            if let Some(s) = scheme {
                next.current_scheme = *s;
            }
            if let Some(t) = theme {
                next.current_theme = t.clone();
            }
            if let Some(v) = variant {
                next.current_variant = v.clone();
            }
        }
        VogixAction::ShaderOn {
            intensity,
            brightness,
            saturation,
        } => {
            let (base_i, base_b, base_s) = match &state.shader {
                ShaderState::On {
                    intensity: i,
                    brightness: b,
                    saturation: s,
                } => (*i, *b, *s),
                _ => (0.5, 1.0, 1.0),
            };
            next.shader = ShaderState::On {
                intensity: intensity.unwrap_or(base_i),
                brightness: brightness.unwrap_or(base_b),
                saturation: saturation.unwrap_or(base_s),
            };
        }
        VogixAction::ShaderOff => {
            next.shader = ShaderState::Off;
        }
        VogixAction::ShaderToggle => {
            next.shader = match &state.shader {
                ShaderState::On { .. } => ShaderState::Off,
                _ => ShaderState::On {
                    intensity: 0.5,
                    brightness: 1.0,
                    saturation: 1.0,
                },
            };
        }
        VogixAction::ShaderParam { param, value } => match &state.shader {
            ShaderState::On {
                intensity,
                brightness,
                saturation,
            } => {
                let (mut i, mut b, mut s) = (*intensity, *brightness, *saturation);
                match param {
                    ShaderParam::Intensity => i = value.clamp(0.0, 1.0),
                    ShaderParam::Brightness => b = value.clamp(0.1, 2.0),
                    ShaderParam::Saturation => s = value.clamp(0.0, 2.0),
                }
                next.shader = ShaderState::On {
                    intensity: i,
                    brightness: b,
                    saturation: s,
                };
            }
            // ShaderMustBeOn precondition ensures this is unreachable
            ShaderState::Off | ShaderState::Auto => {}
        },
        VogixAction::ModeChange { target } => {
            next.current_mode = target.clone();
        }
        VogixAction::Refresh => {
            // No state change — side effects happen post-commit
        }
    }

    Ok(next)
}

// ── Preconditions ──

/// Verify shader parameter values are within valid ranges
pub struct ValidShaderParams;

impl Precondition<VogixAction> for ValidShaderParams {
    fn check(&self, _state: &State, action: &VogixAction) -> Verdict {
        match action {
            VogixAction::ShaderOn {
                intensity,
                brightness,
                saturation,
            } => {
                let mut issues = vec![];
                if let Some(i) = intensity
                    && !(0.0..=1.0).contains(i)
                {
                    issues.push(format!("intensity {:.2} not in [0.0, 1.0]", i));
                }
                if let Some(b) = brightness
                    && !(0.1..=2.0).contains(b)
                {
                    issues.push(format!("brightness {:.2} not in [0.1, 2.0]", b));
                }
                if let Some(s) = saturation
                    && !(0.0..=2.0).contains(s)
                {
                    issues.push(format!("saturation {:.2} not in [0.0, 2.0]", s));
                }
                if issues.is_empty() {
                    Ok(Box::new(SimpleProof::new(meta(
                        "valid_shader_params",
                        "all parameters in range",
                    ))))
                } else {
                    Err(Box::new(SimpleCounterexample::new(meta(
                        "valid_shader_params",
                        &issues.join("; "),
                    ))))
                }
            }
            VogixAction::ShaderParam { param, value } => {
                let valid = match param {
                    ShaderParam::Intensity => (0.0..=1.0).contains(value),
                    ShaderParam::Brightness => (0.1..=2.0).contains(value),
                    ShaderParam::Saturation => (0.0..=2.0).contains(value),
                };
                if valid {
                    Ok(Box::new(SimpleProof::new(meta(
                        "valid_shader_params",
                        "parameter in range",
                    ))))
                } else {
                    Err(Box::new(SimpleCounterexample::new(meta(
                        "valid_shader_params",
                        &format!("{:?} value {:.2} out of range", param, value),
                    ))))
                }
            }
            _ => Ok(Box::new(SimpleProof::new(meta(
                "valid_shader_params",
                "not a shader param action",
            )))),
        }
    }
}

/// Verify shader is on before setting a parameter
pub struct ShaderMustBeOn;

impl Precondition<VogixAction> for ShaderMustBeOn {
    fn check(&self, state: &State, action: &VogixAction) -> Verdict {
        if let VogixAction::ShaderParam { .. } = action {
            if state.shader.is_on() {
                Ok(Box::new(SimpleProof::new(meta(
                    "shader_must_be_on",
                    "shader is on",
                ))))
            } else {
                Err(Box::new(SimpleCounterexample::new(meta(
                    "shader_must_be_on",
                    "cannot set shader param when shader is off",
                ))))
            }
        } else {
            Ok(Box::new(SimpleProof::new(meta(
                "shader_must_be_on",
                "not a shader param action",
            ))))
        }
    }
}

/// Verify mode transition is valid against the ontology ModeGraph.
pub struct ValidModeTransition {
    pub graph: pr4xis_domains::applied::hmi::input::modes::ModeGraph,
}

impl Precondition<VogixAction> for ValidModeTransition {
    fn check(&self, state: &State, action: &VogixAction) -> Verdict {
        if let VogixAction::ModeChange { target } = action {
            let from = pr4xis_domains::applied::hmi::input::modes::ModeId::new(&state.current_mode);
            let to = pr4xis_domains::applied::hmi::input::modes::ModeId::new(target);
            if self.graph.is_valid_transition(&from, &to) {
                Ok(Box::new(SimpleProof::new(meta(
                    "valid_mode_transition",
                    &format!("{} → {}", state.current_mode, target),
                ))))
            } else {
                Err(Box::new(SimpleCounterexample::new(meta(
                    "valid_mode_transition",
                    &format!(
                        "cannot transition from '{}' to '{}'",
                        state.current_mode, target
                    ),
                ))))
            }
        } else {
            Ok(Box::new(SimpleProof::new(meta(
                "valid_mode_transition",
                "not a mode change",
            ))))
        }
    }
}

pub type VogixEngine = Engine<VogixAction>;

pub fn create_engine(state: State) -> VogixEngine {
    create_engine_with_graph(
        state,
        pr4xis_domains::applied::hmi::input::modes::default_mode_graph(),
    )
}

pub fn create_engine_with_graph(
    state: State,
    mode_graph: pr4xis_domains::applied::hmi::input::modes::ModeGraph,
) -> VogixEngine {
    Engine::new(
        state,
        vec![
            Box::new(ValidShaderParams),
            Box::new(ShaderMustBeOn),
            Box::new(ValidModeTransition { graph: mode_graph }),
        ],
        apply_action,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scheme::Scheme;

    fn default_state() -> State {
        State::default()
    }

    fn state_with_shader() -> State {
        State {
            shader: ShaderState::On {
                intensity: 0.5,
                brightness: 1.0,
                saturation: 1.0,
            },
            ..Default::default()
        }
    }

    // ── apply_action tests ──

    #[test]
    fn test_set_theme() {
        let state = default_state();
        let action = VogixAction::SetTheme {
            scheme: Some(Scheme::Base16),
            theme: Some("catppuccin".into()),
            variant: Some("mocha".into()),
        };
        let next = apply_action(&state, &action).unwrap();
        assert_eq!(next.current_scheme, Scheme::Base16);
        assert_eq!(next.current_theme, "catppuccin");
        assert_eq!(next.current_variant, "mocha");
    }

    #[test]
    fn test_set_theme_partial() {
        let state = default_state();
        let action = VogixAction::SetTheme {
            scheme: None,
            theme: Some("gruvbox".into()),
            variant: None,
        };
        let next = apply_action(&state, &action).unwrap();
        assert_eq!(next.current_theme, "gruvbox");
        assert_eq!(next.current_variant, "night"); // unchanged
        assert_eq!(next.current_scheme, Scheme::Vogix16); // unchanged
    }

    #[test]
    fn test_shader_on() {
        let state = default_state();
        let action = VogixAction::ShaderOn {
            intensity: Some(0.3),
            brightness: None,
            saturation: None,
        };
        let next = apply_action(&state, &action).unwrap();
        assert_eq!(
            next.shader,
            ShaderState::On {
                intensity: 0.3,
                brightness: 1.0,
                saturation: 1.0
            }
        );
    }

    #[test]
    fn test_shader_off() {
        let state = state_with_shader();
        let next = apply_action(&state, &VogixAction::ShaderOff).unwrap();
        assert_eq!(next.shader, ShaderState::Off);
    }

    #[test]
    fn test_shader_toggle_on_to_off() {
        let state = state_with_shader();
        let next = apply_action(&state, &VogixAction::ShaderToggle).unwrap();
        assert_eq!(next.shader, ShaderState::Off);
    }

    #[test]
    fn test_shader_toggle_off_to_on() {
        let state = State {
            shader: ShaderState::Off,
            ..Default::default()
        };
        let next = apply_action(&state, &VogixAction::ShaderToggle).unwrap();
        assert!(next.shader.is_on());
    }

    #[test]
    fn test_shader_param_intensity() {
        let state = state_with_shader();
        let action = VogixAction::ShaderParam {
            param: ShaderParam::Intensity,
            value: 0.3,
        };
        let next = apply_action(&state, &action).unwrap();
        assert_eq!(next.shader.params(), Some((0.3, 1.0, 1.0)));
    }

    #[test]
    fn test_shader_param_when_off_is_noop() {
        // apply_action is a no-op when shader is off — ShaderMustBeOn precondition
        // blocks this at the engine level before apply_action is ever called.
        let state = State {
            shader: ShaderState::Off,
            ..Default::default()
        };
        let action = VogixAction::ShaderParam {
            param: ShaderParam::Intensity,
            value: 0.3,
        };
        let next = apply_action(&state, &action).unwrap();
        assert_eq!(next.shader, ShaderState::Off); // unchanged
    }

    #[test]
    fn test_refresh_no_state_change() {
        let state = default_state();
        let next = apply_action(&state, &VogixAction::Refresh).unwrap();
        assert_eq!(state, next);
    }

    // ── Engine tests ──

    #[test]
    fn test_engine_basic_flow() {
        let engine = create_engine(default_state());
        assert_eq!(engine.situation().current_theme, "yoga");

        let engine = engine
            .next(VogixAction::SetTheme {
                scheme: None,
                theme: Some("catppuccin".into()),
                variant: Some("mocha".into()),
            })
            .unwrap();
        assert_eq!(engine.situation().current_theme, "catppuccin");
    }

    #[test]
    fn test_engine_back_forward() {
        let engine = create_engine(default_state());
        let engine = engine
            .next(VogixAction::SetTheme {
                scheme: None,
                theme: Some("gruvbox".into()),
                variant: None,
            })
            .unwrap();
        assert_eq!(engine.situation().current_theme, "gruvbox");

        let engine = engine.back().unwrap();
        assert_eq!(engine.situation().current_theme, "yoga");

        let engine = engine.forward().unwrap();
        assert_eq!(engine.situation().current_theme, "gruvbox");
    }

    #[test]
    fn test_engine_precondition_blocks_invalid_shader_param() {
        let engine = create_engine(State {
            shader: ShaderState::Off,
            ..Default::default()
        });

        let result = engine.next(VogixAction::ShaderParam {
            param: ShaderParam::Intensity,
            value: 0.5,
        });

        assert!(result.is_err());
    }

    #[test]
    fn test_engine_precondition_blocks_out_of_range() {
        let engine = create_engine(state_with_shader());

        let result = engine.next(VogixAction::ShaderOn {
            intensity: Some(5.0), // out of range
            brightness: None,
            saturation: None,
        });

        assert!(result.is_err());
    }

    #[test]
    fn test_engine_shader_on_off_cycle() {
        let engine = create_engine(default_state());

        let engine = engine
            .next(VogixAction::ShaderOn {
                intensity: Some(0.4),
                brightness: None,
                saturation: None,
            })
            .unwrap();
        assert!(engine.situation().shader.is_on());

        let engine = engine.next(VogixAction::ShaderOff).unwrap();
        assert_eq!(engine.situation().shader, ShaderState::Off);

        // Can go back to shader on
        let engine = engine.back().unwrap();
        assert!(engine.situation().shader.is_on());
    }
}

/// Shader parameter ontology — formal ranges and semantics.
///
/// The monochromatic screen shader has three tunable parameters.
/// Each has a defined interval, default value, and semantic meaning.
/// These are NOT magic numbers — they're formal constraints.
///
/// Sources:
/// - GLSL: intensity is a blend factor [0=original, 1=full monochrome]
/// - Rec. 709: brightness is a luminance multiplier
/// - Color science: saturation adjustment on the tint hue
use praxis::ontology::Axiom;
use praxis_domains::science::math::functions::Interval;

/// A shader parameter with formal bounds and semantics.
#[derive(Debug, Clone)]
pub struct ShaderParamDef {
    pub name: &'static str,
    pub description: &'static str,
    pub interval: Interval,
    pub default: f64,
}

/// The three shader parameters, formally defined.
pub fn intensity() -> ShaderParamDef {
    ShaderParamDef {
        name: "intensity",
        description: "Blend factor between original and monochrome [0=none, 1=full]",
        interval: Interval::UNIT, // [0.0, 1.0]
        default: 0.5,
    }
}

pub fn brightness() -> ShaderParamDef {
    ShaderParamDef {
        name: "brightness",
        description: "Luminance multiplier for the monochrome output [0.1=very dark, 2.0=very bright]",
        interval: Interval::new(0.1, 2.0),
        default: 1.0,
    }
}

pub fn saturation() -> ShaderParamDef {
    ShaderParamDef {
        name: "saturation",
        description: "Saturation adjustment on the tint hue [0=gray, 1=normal, 2=vivid]",
        interval: Interval::new(0.0, 2.0),
        default: 1.0,
    }
}

/// All shader parameters.
pub fn all_params() -> Vec<ShaderParamDef> {
    vec![intensity(), brightness(), saturation()]
}

/// Validate a parameter value against its formal interval.
pub fn validate_param(param: &ShaderParamDef, value: f64) -> bool {
    param.interval.contains(value)
}

/// Clamp a value to the parameter's formal interval.
pub fn clamp_param(param: &ShaderParamDef, value: f64) -> f64 {
    param.interval.clamp(value)
}

// ── Axioms ──

/// All defaults are within their intervals.
pub struct DefaultsInRange;

impl Axiom for DefaultsInRange {
    fn description(&self) -> &str {
        "all shader parameter defaults are within their intervals"
    }
    fn holds(&self) -> bool {
        all_params().iter().all(|p| p.interval.contains(p.default))
    }
}

/// Intensity default is 0.5 (balanced blend).
pub struct IntensityBalanced;

impl Axiom for IntensityBalanced {
    fn description(&self) -> &str {
        "intensity default is 0.5 (balanced blend, not full monochrome)"
    }
    fn holds(&self) -> bool {
        (intensity().default - 0.5).abs() < 1e-10
    }
}

/// Brightness default is 1.0 (no change).
pub struct BrightnessNeutral;

impl Axiom for BrightnessNeutral {
    fn description(&self) -> &str {
        "brightness default is 1.0 (neutral, no dimming or boosting)"
    }
    fn holds(&self) -> bool {
        (brightness().default - 1.0).abs() < 1e-10
    }
}

/// Saturation default is 1.0 (no change).
pub struct SaturationNeutral;

impl Axiom for SaturationNeutral {
    fn description(&self) -> &str {
        "saturation default is 1.0 (neutral, no desaturation or boosting)"
    }
    fn holds(&self) -> bool {
        (saturation().default - 1.0).abs() < 1e-10
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_3_params() {
        assert_eq!(all_params().len(), 3);
    }

    #[test]
    fn test_defaults_in_range() {
        assert!(DefaultsInRange.holds());
    }

    #[test]
    fn test_intensity_balanced() {
        assert!(IntensityBalanced.holds());
    }

    #[test]
    fn test_brightness_neutral() {
        assert!(BrightnessNeutral.holds());
    }

    #[test]
    fn test_saturation_neutral() {
        assert!(SaturationNeutral.holds());
    }

    #[test]
    fn test_validate_in_range() {
        assert!(validate_param(&intensity(), 0.5));
        assert!(validate_param(&intensity(), 0.0));
        assert!(validate_param(&intensity(), 1.0));
        assert!(!validate_param(&intensity(), -0.1));
        assert!(!validate_param(&intensity(), 1.1));
    }

    #[test]
    fn test_clamp() {
        assert!((clamp_param(&intensity(), 1.5) - 1.0).abs() < 1e-10);
        assert!((clamp_param(&intensity(), -0.5) - 0.0).abs() < 1e-10);
        assert!((clamp_param(&brightness(), 0.0) - 0.1).abs() < 1e-10);
    }

    // ── Property-based tests ──
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn prop_clamp_always_in_range(value in -10.0f64..10.0) {
            for param in all_params() {
                let clamped = clamp_param(&param, value);
                prop_assert!(param.interval.contains(clamped));
            }
        }

        #[test]
        fn prop_validate_agrees_with_clamp(value in -10.0f64..10.0) {
            for param in all_params() {
                let valid = validate_param(&param, value);
                let clamped = clamp_param(&param, value);
                if valid {
                    prop_assert!((clamped - value).abs() < 1e-10, "valid value should not be clamped");
                }
            }
        }
    }
}

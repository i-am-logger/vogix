//! Theme and variant type definitions.

use crate::errors::{Result, VogixError};
use crate::scheme::Scheme;

/// Variant information with polarity and order (0 = lightest)
#[derive(Debug, Clone)]
pub struct VariantInfo {
    pub name: String,
    pub polarity: String,
    pub order: u32,
}

/// Theme information from the config manifest
#[derive(Debug, Clone)]
pub struct ThemeInfo {
    pub name: String,
    pub scheme: Scheme,
    pub variants: Vec<VariantInfo>,
}

impl ThemeInfo {
    /// Get variants sorted by order (lightest first, order 0)
    pub fn variants_by_order(&self) -> Vec<&VariantInfo> {
        let mut sorted: Vec<_> = self.variants.iter().collect();
        sorted.sort_by_key(|v| v.order);
        sorted
    }

    /// Navigate to darker or lighter variant
    /// Returns the new variant name, or error if at boundary
    pub fn navigate(&self, current: &str, direction: &str) -> Result<String> {
        let sorted = self.variants_by_order();

        // Find current position
        let current_idx = sorted
            .iter()
            .position(|v| v.name.to_lowercase() == current.to_lowercase())
            .ok_or_else(|| {
                VogixError::InvalidTheme(format!("Variant '{}' not found in theme", current))
            })?;

        match direction.to_lowercase().as_str() {
            "darker" => {
                if current_idx >= sorted.len() - 1 {
                    Err(VogixError::InvalidTheme(
                        "Already at darkest variant".to_string(),
                    ))
                } else {
                    Ok(sorted[current_idx + 1].name.clone())
                }
            }
            "lighter" => {
                if current_idx == 0 {
                    Err(VogixError::InvalidTheme(
                        "Already at lightest variant".to_string(),
                    ))
                } else {
                    Ok(sorted[current_idx - 1].name.clone())
                }
            }
            _ => Err(VogixError::InvalidTheme(format!(
                "Unknown direction: {}. Use 'darker' or 'lighter'",
                direction
            ))),
        }
    }

    /// Normalized luminance position of `variant_name`: 0.0 = lightest, 1.0 =
    /// darkest. Single-variant themes return 0.0. Used to match illumination
    /// across themes on a switch.
    pub fn order_fraction(&self, variant_name: &str) -> Option<f64> {
        let sorted = self.variants_by_order();
        let max = sorted.len().saturating_sub(1);
        let idx = sorted
            .iter()
            .position(|v| v.name.eq_ignore_ascii_case(variant_name))?;
        Some(if max == 0 {
            0.0
        } else {
            idx as f64 / max as f64
        })
    }

    /// The variant whose normalized luminance position is closest to `frac`
    /// (0.0 = lightest .. 1.0 = darkest). Ties keep the lighter variant.
    pub fn nearest_by_fraction(&self, frac: f64) -> &VariantInfo {
        let sorted = self.variants_by_order();
        let max = sorted.len().saturating_sub(1);
        let mut best = sorted[0];
        let mut best_d = f64::INFINITY;
        for (i, v) in sorted.iter().enumerate() {
            let pos = if max == 0 { 0.0 } else { i as f64 / max as f64 };
            let d = (pos - frac).abs();
            if d < best_d {
                best_d = d;
                best = *v;
            }
        }
        best
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_variant_info_creation() {
        let variant = VariantInfo {
            name: "dark".to_string(),
            polarity: "dark".to_string(),
            order: 1,
        };
        assert_eq!(variant.name, "dark");
        assert_eq!(variant.order, 1);
    }

    #[test]
    fn test_theme_navigate_darker() {
        let theme = ThemeInfo {
            name: "test".to_string(),
            scheme: Scheme::Vogix16,
            variants: vec![
                VariantInfo {
                    name: "light".to_string(),
                    polarity: "light".to_string(),
                    order: 0, // lightest
                },
                VariantInfo {
                    name: "dark".to_string(),
                    polarity: "dark".to_string(),
                    order: 1, // darkest
                },
            ],
        };

        let result = theme.navigate("light", "darker").unwrap();
        assert_eq!(result, "dark");
    }

    #[test]
    fn test_theme_navigate_lighter() {
        let theme = ThemeInfo {
            name: "test".to_string(),
            scheme: Scheme::Vogix16,
            variants: vec![
                VariantInfo {
                    name: "light".to_string(),
                    polarity: "light".to_string(),
                    order: 0,
                },
                VariantInfo {
                    name: "dark".to_string(),
                    polarity: "dark".to_string(),
                    order: 1,
                },
            ],
        };

        let result = theme.navigate("dark", "lighter").unwrap();
        assert_eq!(result, "light");
    }

    #[test]
    fn test_theme_navigate_at_boundary() {
        let theme = ThemeInfo {
            name: "test".to_string(),
            scheme: Scheme::Vogix16,
            variants: vec![
                VariantInfo {
                    name: "light".to_string(),
                    polarity: "light".to_string(),
                    order: 0,
                },
                VariantInfo {
                    name: "dark".to_string(),
                    polarity: "dark".to_string(),
                    order: 1,
                },
            ],
        };

        // Already at darkest
        assert!(theme.navigate("dark", "darker").is_err());

        // Already at lightest
        assert!(theme.navigate("light", "lighter").is_err());
    }

    #[test]
    fn test_theme_navigate_multi_variant() {
        // Rose-pine style: dawn (lightest, order=0), moon (order=1), base (darkest, order=2)
        let theme = ThemeInfo {
            name: "rose-pine".to_string(),
            scheme: Scheme::Base16,
            variants: vec![
                VariantInfo {
                    name: "dawn".to_string(),
                    polarity: "light".to_string(),
                    order: 0,
                },
                VariantInfo {
                    name: "moon".to_string(),
                    polarity: "dark".to_string(),
                    order: 1,
                },
                VariantInfo {
                    name: "base".to_string(),
                    polarity: "dark".to_string(),
                    order: 2,
                },
            ],
        };

        // Navigate from dawn (lightest) -> moon -> base (darkest)
        let result1 = theme.navigate("dawn", "darker").unwrap();
        assert_eq!(result1, "moon");

        let result2 = theme.navigate("moon", "darker").unwrap();
        assert_eq!(result2, "base");

        // Can't go darker than base
        assert!(theme.navigate("base", "darker").is_err());

        // Navigate back: base -> moon -> dawn
        let result3 = theme.navigate("base", "lighter").unwrap();
        assert_eq!(result3, "moon");

        let result4 = theme.navigate("moon", "lighter").unwrap();
        assert_eq!(result4, "dawn");

        // Can't go lighter than dawn
        assert!(theme.navigate("dawn", "lighter").is_err());
    }

    #[test]
    fn test_variants_by_order() {
        let theme = ThemeInfo {
            name: "test".to_string(),
            scheme: Scheme::Base16,
            variants: vec![
                VariantInfo {
                    name: "base".to_string(),
                    polarity: "dark".to_string(),
                    order: 2, // darkest
                },
                VariantInfo {
                    name: "dawn".to_string(),
                    polarity: "light".to_string(),
                    order: 0, // lightest
                },
                VariantInfo {
                    name: "moon".to_string(),
                    polarity: "dark".to_string(),
                    order: 1,
                },
            ],
        };

        let sorted = theme.variants_by_order();
        // Should be sorted by order: dawn (0) first, base (2) last
        assert_eq!(sorted[0].name, "dawn");
        assert_eq!(sorted[1].name, "moon");
        assert_eq!(sorted[2].name, "base");
    }

    fn rose_pine() -> ThemeInfo {
        // dawn (lightest, 0), moon (dark, 1), base (darkest, 2) — two darks.
        ThemeInfo {
            name: "rose-pine".to_string(),
            scheme: Scheme::Base16,
            variants: vec![
                VariantInfo {
                    name: "dawn".to_string(),
                    polarity: "light".to_string(),
                    order: 0,
                },
                VariantInfo {
                    name: "moon".to_string(),
                    polarity: "dark".to_string(),
                    order: 1,
                },
                VariantInfo {
                    name: "base".to_string(),
                    polarity: "dark".to_string(),
                    order: 2,
                },
            ],
        }
    }

    #[test]
    fn test_order_fraction() {
        let theme = rose_pine();
        assert_eq!(theme.order_fraction("dawn"), Some(0.0)); // lightest
        assert_eq!(theme.order_fraction("moon"), Some(0.5)); // middle
        assert_eq!(theme.order_fraction("base"), Some(1.0)); // darkest
        assert_eq!(theme.order_fraction("DAWN"), Some(0.0)); // case-insensitive
        assert_eq!(theme.order_fraction("nope"), None);
    }

    #[test]
    fn test_nearest_by_fraction() {
        // Two-variant theme (yoga-style): day (light, 0), night (dark, 1).
        let two = ThemeInfo {
            name: "yoga".to_string(),
            scheme: Scheme::Vogix16,
            variants: vec![
                VariantInfo {
                    name: "day".to_string(),
                    polarity: "light".to_string(),
                    order: 0,
                },
                VariantInfo {
                    name: "night".to_string(),
                    polarity: "dark".to_string(),
                    order: 1,
                },
            ],
        };
        // From the dark end -> night; from the light end -> day (polarity-preserving).
        assert_eq!(two.nearest_by_fraction(1.0).name, "night");
        assert_eq!(two.nearest_by_fraction(0.0).name, "day");
        assert_eq!(two.nearest_by_fraction(0.4).name, "day");
        assert_eq!(two.nearest_by_fraction(0.6).name, "night");

        // Multi-variant: a mid source lands on the nearest-luminance variant.
        let rp = rose_pine();
        assert_eq!(rp.nearest_by_fraction(0.5).name, "moon"); // exact middle
        assert_eq!(rp.nearest_by_fraction(1.0).name, "base"); // darkest
        assert_eq!(rp.nearest_by_fraction(0.0).name, "dawn"); // lightest
    }
}

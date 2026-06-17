//! Theme change helpers — variant resolution and navigation.
//!
//! The actual theme change flow goes through the praxis engine
//! (cli_to_action → engine.next → execute_side_effects in main.rs).
//! This module provides the filesystem-aware resolution functions.

use crate::errors::{Result, VogixError};
use crate::state::State;
use crate::theme;

/// On a theme switch with no explicit variant, match the current illumination:
/// pick the new theme's variant whose normalized luminance rank is closest to
/// the current variant's (falling back to the dark end if unknown). This keeps
/// brightness consistent across themes; for the common two-variant theme it is
/// identical to polarity-matching (dark→dark, light→light).
pub fn resolve_illumination_variant(state: &State, new_theme: &str) -> Result<Option<String>> {
    let themes = theme::discover_themes()?;
    let new_info = theme::get_theme(&themes, new_theme)
        .ok_or_else(|| VogixError::InvalidTheme(format!("Theme '{}' not found", new_theme)))?;

    if new_info.variants.is_empty() {
        return Err(VogixError::InvalidTheme(format!(
            "Theme '{}' has no variants",
            new_theme
        )));
    }

    let current_frac = theme::get_theme(&themes, &state.current_theme)
        .and_then(|t| t.order_fraction(&state.current_variant))
        .unwrap_or(1.0);

    Ok(Some(
        new_info.nearest_by_fraction(current_frac).name.clone(),
    ))
}

/// Direction along the luminance ramp.
enum Direction {
    Lighter,
    Darker,
}

/// Parse a directional `-v` request. `light`/`lighter` move toward the lightest
/// variant; `dark`/`darker` toward the darkest. Returns `None` for an exact name.
fn parse_direction(requested_lower: &str) -> Option<Direction> {
    match requested_lower {
        "light" | "lighter" => Some(Direction::Lighter),
        "dark" | "darker" => Some(Direction::Darker),
        _ => None,
    }
}

/// Resolve a `-v` request for `theme_name`.
///
/// - exact variant name → that variant;
/// - directional (`light`/`lighter`/`dark`/`darker`):
///   - single-variant theme → its only variant;
///   - same theme (`!is_switch`) → step one along the luminance ramp from
///     `current_variant`, erroring at the boundary;
///   - theme switch (`is_switch`) → that theme's lightest/darkest end, since
///     there is no current position in the new theme to step from.
pub fn resolve_variant(
    theme_name: &str,
    requested: &str,
    current_variant: &str,
    is_switch: bool,
) -> Result<String> {
    let themes = theme::discover_themes()?;
    let theme_info = theme::get_theme(&themes, theme_name)
        .ok_or_else(|| VogixError::InvalidTheme(format!("Theme '{}' not found", theme_name)))?;

    let requested_lower = requested.to_lowercase();

    // Exact variant name match.
    if let Some(variant) = theme_info
        .variants
        .iter()
        .find(|v| v.name.to_lowercase() == requested_lower)
    {
        return Ok(variant.name.clone());
    }

    match parse_direction(&requested_lower) {
        Some(direction) => {
            // A theme with no variants can't be resolved (degenerate manifest).
            if theme_info.variants.is_empty() {
                return Err(VogixError::InvalidTheme(format!(
                    "Theme '{}' has no variants",
                    theme_name
                )));
            }
            // Single-variant theme: nowhere to step — use the only variant.
            if theme_info.variants.len() == 1 {
                return Ok(theme_info.variants[0].name.clone());
            }
            if is_switch {
                // No current position in the new theme; jump to that ramp end.
                let sorted = theme_info.variants_by_order(); // lightest (order 0) first
                let pick = match direction {
                    Direction::Lighter => sorted.first(),
                    Direction::Darker => sorted.last(),
                };
                // Non-empty was checked above, so a variant always exists.
                Ok(pick.expect("variants is non-empty").name.clone())
            } else {
                // Same theme: step one along the ramp from the current variant.
                let dir = match direction {
                    Direction::Lighter => "lighter",
                    Direction::Darker => "darker",
                };
                theme_info.navigate(current_variant, dir)
            }
        }
        None => {
            let available: Vec<_> = theme_info
                .variants
                .iter()
                .map(|v| v.name.as_str())
                .collect();
            Err(VogixError::InvalidTheme(format!(
                "Variant '{}' not found in theme '{}'. Available variants: {}",
                requested,
                theme_name,
                available.join(", ")
            )))
        }
    }
}

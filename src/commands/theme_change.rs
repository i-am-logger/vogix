//! Theme change helpers — variant resolution and navigation.
//!
//! The actual theme change flow goes through the praxis engine
//! (cli_to_action → engine.next → execute_side_effects in main.rs).
//! This module provides the filesystem-aware resolution functions.

use crate::errors::{Result, VogixError};
use crate::state::State;
use crate::theme;

/// Resolve the best variant for a new theme, matching the current polarity.
pub fn resolve_polarity_variant(state: &State, new_theme: &str) -> Result<Option<String>> {
    let themes = theme::discover_themes()?;
    let new_theme_info = match theme::get_theme(&themes, new_theme) {
        Some(t) => t,
        None => return Ok(None),
    };

    let current_polarity = theme::get_theme(&themes, &state.current_theme)
        .and_then(|t| {
            t.variants
                .iter()
                .find(|v| v.name == state.current_variant)
                .map(|v| v.polarity.clone())
        })
        .unwrap_or_else(|| "dark".to_string());

    if let Some(default_var) = new_theme_info.default_variant_for_polarity(&current_polarity) {
        Ok(Some(default_var.name.clone()))
    } else {
        Ok(None)
    }
}

/// Navigate to a darker or lighter variant based on luminance ordering.
pub fn navigate_variant(state: &State, direction: &str) -> Result<String> {
    let themes = theme::discover_themes()?;
    let current_theme = theme::get_theme(&themes, &state.current_theme).ok_or_else(|| {
        VogixError::InvalidTheme(format!("Theme '{}' not found", state.current_theme))
    })?;
    current_theme.navigate(&state.current_variant, direction)
}

/// Resolve a variant name: exact match, polarity request (dark/light), or error.
pub fn resolve_variant(
    theme_name: &str,
    requested: &str,
    _current_variant: &str,
) -> Result<String> {
    let themes = theme::discover_themes()?;
    let theme_info = theme::get_theme(&themes, theme_name)
        .ok_or_else(|| VogixError::InvalidTheme(format!("Theme '{}' not found", theme_name)))?;

    let requested_lower = requested.to_lowercase();

    // Exact variant name match
    for variant in &theme_info.variants {
        if variant.name.to_lowercase() == requested_lower {
            return Ok(variant.name.clone());
        }
    }

    // Single-variant themes: always use the only variant
    if theme_info.variants.len() == 1 {
        return Ok(theme_info.variants[0].name.clone());
    }

    // Polarity request (dark/light)
    if requested_lower == "dark" || requested_lower == "light" {
        if let Some(variant) = theme_info.default_variant_for_polarity(&requested_lower) {
            if variant.polarity == requested_lower {
                return Ok(variant.name.clone());
            }
        }
        let available_polarities: Vec<_> = theme_info
            .variants
            .iter()
            .map(|v| format!("{} ({})", v.name, v.polarity))
            .collect();
        return Err(VogixError::InvalidTheme(format!(
            "Theme '{}' has no '{}' variant. Available: {}",
            theme_name,
            requested,
            available_polarities.join(", ")
        )));
    }

    // Not found
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

//! Theme change command - handle -t, -v, -s flags.

use crate::cli::is_variant_navigation;
use crate::config::Config;
use crate::errors::{Result, VogixError};
use crate::reload::ReloadDispatcher;
use crate::scheme::Scheme;
use crate::state::State;
use crate::symlink::SymlinkManager;
use crate::theme;
use log::{debug, info, warn};

use super::refresh::maybe_render_templates;

/// Handle theme/variant/scheme changes
pub fn handle_theme_change(
    scheme: Option<Scheme>,
    theme: Option<String>,
    variant: Option<String>,
    quiet: bool,
) -> Result<()> {
    let mut state = State::load()?;
    let config = Config::load()?;

    let old_scheme = state.current_scheme;
    let old_theme = state.current_theme.clone();
    let old_variant = state.current_variant.clone();

    // Update scheme if provided
    if let Some(scheme) = scheme {
        state.current_scheme = scheme;
    }

    // Track if theme changed (we'll need to resolve variant for new theme)
    let theme_changed = theme.is_some() && theme.as_ref() != Some(&state.current_theme);

    // Update theme if provided
    if let Some(theme) = &theme {
        state.current_theme = theme.clone();
    }

    // Update variant if provided, OR resolve default variant if theme changed
    if let Some(variant) = &variant {
        if is_variant_navigation(&Some(variant.clone())) {
            state.current_variant = navigate_variant(&state, variant)?;
        } else {
            // Resolve variant: could be an exact variant name OR a polarity request (dark/light)
            state.current_variant =
                resolve_variant(&state.current_theme, variant, &state.current_variant)?;
        }
    } else if theme_changed {
        // Theme changed but no variant specified - find appropriate variant for current polarity
        let themes = theme::discover_themes()?;
        if let Some(new_theme) = theme::get_theme(&themes, &state.current_theme) {
            // Get the polarity of the old variant to maintain dark/light preference
            let current_polarity = theme::get_theme(&themes, &old_theme)
                .and_then(|t| {
                    t.variants
                        .iter()
                        .find(|v| v.name == old_variant)
                        .map(|v| v.polarity.clone())
                })
                .unwrap_or_else(|| "dark".to_string());

            // Find the default variant for this polarity in the new theme
            if let Some(default_var) = new_theme.default_variant_for_polarity(&current_polarity) {
                state.current_variant = default_var.name.clone();
            }
        }
    }

    // Check if anything changed
    if state.current_scheme == old_scheme
        && state.current_theme == old_theme
        && state.current_variant == old_variant
    {
        info!("No changes to apply");
        return Ok(());
    }

    // Log changes
    if state.current_scheme != old_scheme {
        info!("scheme: {} → {}", old_scheme, state.current_scheme);
    }
    if state.current_theme != old_theme {
        info!("theme: {} → {}", old_theme, state.current_theme);
    }
    if state.current_variant != old_variant {
        info!("variant: {} → {}", old_variant, state.current_variant);
    }

    // Verify theme-variant exists
    theme::verify_theme_variant_exists(&state.current_theme, &state.current_variant)?;
    debug!("Verified theme-variant exists");

    // Render templates to cache if configured (for template-based architecture)
    if let Some(cache_path) = maybe_render_templates(&config, &state)? {
        debug!(
            "Using template-rendered configs from: {}",
            cache_path.display()
        );
    }

    // Update the 'current' symlink
    let symlink_manager = SymlinkManager::new();
    symlink_manager.update_current_symlink(&state.current_theme, &state.current_variant)?;
    debug!("Updated current symlink");

    // Save state
    state.save()?;
    debug!("Saved state");

    // Reload applications
    let reload_dispatcher = ReloadDispatcher::new();
    let reload_result = reload_dispatcher.reload_apps(&config, quiet);

    // Apply theme colors to hardware devices
    if !config.hardware.is_empty()
        && let Some(theme_sources) = &config.theme_sources
    {
        let variant_path = crate::cache::paths::theme_variant_path(
            theme_sources,
            &state.current_scheme,
            &state.current_theme,
            &state.current_variant,
        );
        match theme::load_theme_colors(&variant_path, state.current_scheme) {
            Ok(colors) => reload_dispatcher.apply_hardware(&config, &colors, quiet),
            Err(e) => warn!("Hardware theme apply skipped: {}", e),
        }
    }

    // Auto-apply shader if configured (updates tint color for new theme)
    if let Err(e) = super::shader::maybe_apply_shader(&config, &state) {
        warn!("Shader apply failed: {}", e);
    }

    // Log appropriate message based on reload results
    let theme_variant = format!("{}-{}", state.current_theme, state.current_variant);
    if reload_result.has_failures() {
        warn!(
            "Applied: {} ({}/{} reloaded, {} failed)",
            theme_variant,
            reload_result.success_count,
            reload_result.total_count,
            reload_result.failed_apps.len()
        );
    } else {
        info!("Applied: {}", theme_variant);
    }
    Ok(())
}

/// Resolve the best variant for a new theme, matching the current polarity.
/// Returns None if the theme doesn't exist or has no matching variant.
pub fn resolve_polarity_variant(state: &State, new_theme: &str) -> Result<Option<String>> {
    let themes = theme::discover_themes()?;
    let new_theme_info = match theme::get_theme(&themes, new_theme) {
        Some(t) => t,
        None => return Ok(None),
    };

    // Get polarity of current variant
    let current_polarity = theme::get_theme(&themes, &state.current_theme)
        .and_then(|t| {
            t.variants
                .iter()
                .find(|v| v.name == state.current_variant)
                .map(|v| v.polarity.clone())
        })
        .unwrap_or_else(|| "dark".to_string());

    // Find matching variant in new theme
    if let Some(default_var) = new_theme_info.default_variant_for_polarity(&current_polarity) {
        Ok(Some(default_var.name.clone()))
    } else {
        Ok(None)
    }
}

/// Navigate to a darker or lighter variant based on luminance ordering
pub fn navigate_variant(state: &State, direction: &str) -> Result<String> {
    // Load themes and find the current one
    let themes = theme::discover_themes()?;
    let current_theme = theme::get_theme(&themes, &state.current_theme).ok_or_else(|| {
        VogixError::InvalidTheme(format!("Theme '{}' not found", state.current_theme))
    })?;

    // Use the theme's luminance-based navigation
    current_theme.navigate(&state.current_variant, direction)
}

/// Resolve a variant name: could be an exact variant name OR a polarity request (dark/light)
/// For polarity requests, finds the default variant for that polarity in the theme.
/// For single-variant themes, always returns the only variant (ignores polarity request).
pub fn resolve_variant(
    theme_name: &str,
    requested: &str,
    _current_variant: &str,
) -> Result<String> {
    let themes = theme::discover_themes()?;
    let theme_info = theme::get_theme(&themes, theme_name)
        .ok_or_else(|| VogixError::InvalidTheme(format!("Theme '{}' not found", theme_name)))?;

    let requested_lower = requested.to_lowercase();

    // First, check if requested is an exact variant name match (case-insensitive)
    for variant in &theme_info.variants {
        if variant.name.to_lowercase() == requested_lower {
            return Ok(variant.name.clone());
        }
    }

    // For single-variant themes, always use the only variant regardless of polarity request
    if theme_info.variants.len() == 1 {
        return Ok(theme_info.variants[0].name.clone());
    }

    // If not an exact match, check if it's a polarity request (dark/light)
    if requested_lower == "dark" || requested_lower == "light" {
        // Find the default variant for this polarity
        if let Some(variant) = theme_info.default_variant_for_polarity(&requested_lower) {
            // Verify the variant actually has the requested polarity
            if variant.polarity == requested_lower {
                return Ok(variant.name.clone());
            }
        }

        // No variant with the requested polarity exists
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

    // Not an exact match and not a polarity - invalid variant
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

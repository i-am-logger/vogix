//! Template rendering functions
//!
//! Core rendering logic using Tera template engine.

use super::filters;
use crate::errors::{Result, VogixError};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use tera::{Context, Tera};

/// Render a template file with the given colors and theme identity
///
/// Templates use Tera/Jinja2 syntax:
/// ```text
/// background = "{{ colors.base00 }}"
/// foreground = "{{ colors.base05 }}"
/// theme = "{{ meta.theme }}"
/// ```
pub fn render_template<P: AsRef<Path>>(
    template_path: P,
    colors: &HashMap<String, String>,
    meta: &HashMap<String, String>,
) -> Result<String> {
    let template_path = template_path.as_ref();
    let template_content = fs::read_to_string(template_path)
        .map_err(|_| VogixError::ConfigNotFound(template_path.to_path_buf()))?;

    render_template_string(&template_content, colors, meta)
}

/// Render a template string with the given colors and theme identity
///
/// `meta` carries theme/variant/scheme/polarity (the contract identity the
/// vogix-desktop `theme.json` template embeds); templates that don't
/// reference `meta.*` are unaffected.
///
/// This is useful for testing without file I/O.
///
/// # Available filters
/// - `hex_to_rgb`: Convert "#RRGGBB" to "0xRR,0xGG,0xBB" (for ripgrep)
/// - `strip_hash`: Convert "#RRGGBB" to "RRGGBB"
pub fn render_template_string(
    template_content: &str,
    colors: &HashMap<String, String>,
    meta: &HashMap<String, String>,
) -> Result<String> {
    let mut tera = Tera::default();

    // Register custom filters
    tera.register_filter("hex_to_rgb", filters::hex_to_rgb);
    tera.register_filter("strip_hash", filters::strip_hash);

    tera.add_raw_template("template", template_content)
        .map_err(VogixError::Template)?;

    let mut context = Context::new();
    context.insert("colors", colors);
    context.insert("meta", meta);

    tera.render("template", &context)
        .map_err(VogixError::Template)
}

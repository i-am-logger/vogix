//! List command - show available themes and schemes.

use crate::errors::Result;
use crate::scheme::Scheme;
use crate::theme;
use crate::theme::types::ThemeInfo;
use log::info;
use std::borrow::Cow;

/// Handle the `list` command - display themes and schemes
pub fn handle_list(filter_scheme: Option<&Scheme>, show_variants: bool) -> Result<()> {
    let all_themes = theme::discover_themes()?;

    if all_themes.is_empty() {
        info!("No themes found");
        info!("Add themes to your NixOS/home-manager configuration");
        return Ok(());
    }

    // Filter by scheme if provided, avoiding clone when no filter
    let themes: Cow<'_, [ThemeInfo]> = if let Some(scheme) = filter_scheme {
        Cow::Owned(theme::filter_by_scheme(&all_themes, scheme))
    } else {
        Cow::Borrowed(&all_themes)
    };

    // Show available schemes if no filter
    if filter_scheme.is_none() {
        // Count themes per scheme
        let vogix16_count = theme::filter_by_scheme(&all_themes, &Scheme::Vogix16).len();
        let base16_count = theme::filter_by_scheme(&all_themes, &Scheme::Base16).len();
        let base24_count = theme::filter_by_scheme(&all_themes, &Scheme::Base24).len();
        let ansi16_count = theme::filter_by_scheme(&all_themes, &Scheme::Ansi16).len();

        println!("Schemes:");
        println!("  vogix16 ({} themes)", vogix16_count);
        println!("  base16  ({} themes)", base16_count);
        println!("  base24  ({} themes)", base24_count);
        println!("  ansi16  ({} themes)", ansi16_count);
        println!();
        println!("Use 'vogix theme list -s <scheme>' to list themes for a specific scheme");
        println!();
    }

    if themes.is_empty() {
        if let Some(scheme) = filter_scheme {
            info!("No themes found for scheme: {}", scheme);
        }
        return Ok(());
    }

    println!(
        "Themes{}:",
        filter_scheme
            .map(|s| format!(" ({})", s))
            .unwrap_or_default()
    );

    for t in themes.iter() {
        if show_variants {
            // Show variants with polarity info: name(polarity)
            let variant_info: Vec<String> = t
                .variants_by_order()
                .iter()
                .map(|v| format!("{}({})", v.name, v.polarity))
                .collect();
            println!("  {} [{}]", t.name, variant_info.join(", "));
        } else {
            println!("  {}", t.name);
        }
    }

    println!();
    println!("Total: {}", themes.len());

    Ok(())
}

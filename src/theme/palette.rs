//! Bridge between vogix theme loading and praxis theming ontology.
//!
//! Converts raw HashMap<String, String> (key→hex) from the theme loader
//! into a praxis Palette (ColorSlot→Rgb) for ontological reasoning.
//!
//! Sources:
//! - Base16: tinted-theming/home styling.md (key format)
//! - Base24: tinted-theming/base24 styling.md (key format)
//! - Vogix16: vogix design system (semantic names → base16 slots)
//! - Ansi16: ECMA-48 Section 8.3.117 (terminal color indices)
//! - WCAG 2.1: contrast validation axioms

use crate::scheme::Scheme;
use pr4xis::category::FinitelyGenerated;
use pr4xis_domains::applied::hmi::theming::base16::{ColorSlot, Polarity};
use pr4xis_domains::applied::hmi::theming::ontology::{self, Palette};
use pr4xis_domains::applied::hmi::theming::schemes::Ansi16Color;
use pr4xis_domains::natural::colors::Rgb;
use std::collections::HashMap;

/// Try to insert a color into the palette from a hex string.
fn insert_hex(palette: &mut Palette, slot: ColorSlot, hex: &str) {
    if let Some(rgb) = Rgb::from_hex(hex) {
        palette.insert(slot, rgb);
    }
}

/// Build a praxis Palette from raw theme colors.
///
/// Handles all scheme naming conventions using the praxis ontology
/// to map keys to ColorSlots.
pub fn build_palette(colors: &HashMap<String, String>, scheme: Scheme) -> Palette {
    let mut palette = Palette::new();

    match scheme {
        // vogix16 theme files are base16-shaped (base00..0F): the loader keeps
        // those keys and only ADDS semantic aliases, so all three read by slot.
        // The vogix16 *interpretation* (base08 = success, …) is the consumer's
        // concern; the palette is the same ColorSlot→Rgb map regardless of scheme.
        Scheme::Base16 | Scheme::Base24 | Scheme::Vogix16 => {
            for slot in ColorSlot::variants() {
                if let Some(hex) = colors.get(slot.key()) {
                    insert_hex(&mut palette, slot, hex);
                }
            }
        }
        Scheme::Ansi16 => {
            for ansi in Ansi16Color::variants() {
                if let Some(hex) = colors.get(&ansi.key()) {
                    insert_hex(&mut palette, ansi.to_base16_slot(), hex);
                }
            }
            // Special keys for background/foreground
            if let Some(hex) = colors.get("background") {
                insert_hex(&mut palette, ColorSlot::Base00, hex);
            }
            if let Some(hex) = colors.get("foreground") {
                insert_hex(&mut palette, ColorSlot::Base05, hex);
            }
        }
    }

    palette
}

/// Detect theme polarity using the praxis ontology.
pub fn polarity(palette: &Palette) -> Option<Polarity> {
    ontology::detect_polarity(palette)
}

/// The functional (accent) colors of a palette, in `ColorSlot` order, for every
/// slot whose role is `Accent` or `BrightAccent`. These are the semantic colors
/// the screen shader preserves through the monochrome tint — selected by ontology
/// role, so the choice is scheme-correct and hue-agnostic (it works for vogix16's
/// editorial accent hues just as well as base16's). Deterministic order keeps the
/// generated shader byte-stable for caching.
pub fn functional_colors(palette: &Palette) -> Vec<Rgb> {
    use pr4xis_domains::applied::hmi::theming::base16::SemanticRole;
    ColorSlot::variants()
        .into_iter()
        .filter(|slot| {
            matches!(
                slot.role(),
                SemanticRole::Accent | SemanticRole::BrightAccent
            )
        })
        .filter_map(|slot| palette.get(&slot).copied())
        .collect()
}

/// Validate palette against praxis axioms.
/// Returns list of failed axiom descriptions, empty if all pass.
pub fn validate(palette: &Palette) -> Vec<String> {
    use pr4xis::ontology::Axiom;
    use pr4xis_domains::applied::hmi::theming::ontology::{
        LuminanceMonotonicity, WcagForegroundContrast,
    };

    let mut failures = Vec::new();

    let mono = LuminanceMonotonicity {
        palette: palette.clone(),
    };
    if mono.verify().is_err() {
        failures.push(mono.description().as_str().to_string());
    }

    let contrast = WcagForegroundContrast {
        palette: palette.clone(),
    };
    if contrast.verify().is_err() {
        failures.push(contrast.description().as_str().to_string());
    }

    failures
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base16_colors() -> HashMap<String, String> {
        let mut c = HashMap::new();
        c.insert("base00".into(), "#1e1e2e".into());
        c.insert("base01".into(), "#313244".into());
        c.insert("base02".into(), "#45475a".into());
        c.insert("base03".into(), "#585b70".into());
        c.insert("base04".into(), "#6c7086".into());
        c.insert("base05".into(), "#cdd6f4".into());
        c.insert("base06".into(), "#d8dee8".into());
        c.insert("base07".into(), "#eceff4".into());
        c.insert("base08".into(), "#f38ba8".into());
        c.insert("base09".into(), "#fab387".into());
        c.insert("base0A".into(), "#f9e2af".into());
        c.insert("base0B".into(), "#a6e3a1".into());
        c.insert("base0C".into(), "#94e2d5".into());
        c.insert("base0D".into(), "#89b4fa".into());
        c.insert("base0E".into(), "#cba6f7".into());
        c.insert("base0F".into(), "#f2cdcd".into());
        c
    }

    // vogix16 theme files are base16-shaped (base00..0F). Accents carry semantic
    // ROLES (base08=success, base0B=danger) with theme-chosen hues — Western here
    // (success green, danger red), matching the real `yoga` theme.
    fn vogix16_colors() -> HashMap<String, String> {
        let mut c = base16_colors();
        c.insert("base08".into(), "#2d8a2d".into()); // success = green
        c.insert("base0B".into(), "#c23030".into()); // danger  = red
        c
    }

    fn ansi16_colors() -> HashMap<String, String> {
        let mut c = HashMap::new();
        c.insert("color00".into(), "#1e1e2e".into());
        c.insert("color01".into(), "#f38ba8".into());
        c.insert("color02".into(), "#a6e3a1".into());
        c.insert("color03".into(), "#f9e2af".into());
        c.insert("color04".into(), "#89b4fa".into());
        c.insert("color05".into(), "#cba6f7".into());
        c.insert("color06".into(), "#94e2d5".into());
        c.insert("color07".into(), "#cdd6f4".into());
        c.insert("color08".into(), "#585b70".into());
        c.insert("color09".into(), "#f38ba8".into());
        c.insert("color10".into(), "#a6e3a1".into());
        c.insert("color11".into(), "#f9e2af".into());
        c.insert("color12".into(), "#89b4fa".into());
        c.insert("color13".into(), "#cba6f7".into());
        c.insert("color14".into(), "#94e2d5".into());
        c.insert("color15".into(), "#eceff4".into());
        c
    }

    #[test]
    fn test_base16_palette_16_slots() {
        let palette = build_palette(&base16_colors(), Scheme::Base16);
        assert_eq!(palette.len(), 16);
    }

    #[test]
    fn test_vogix16_palette_16_slots() {
        let palette = build_palette(&vogix16_colors(), Scheme::Vogix16);
        assert_eq!(palette.len(), 16);
    }

    #[test]
    fn test_ansi16_palette_has_slots() {
        let palette = build_palette(&ansi16_colors(), Scheme::Ansi16);
        assert!(palette.len() >= 8);
    }

    #[test]
    fn test_base16_and_vogix16_same_palette() {
        let p1 = build_palette(&base16_colors(), Scheme::Base16);
        let p2 = build_palette(&vogix16_colors(), Scheme::Vogix16);
        assert_eq!(p1.get(&ColorSlot::Base00), p2.get(&ColorSlot::Base00));
        assert_eq!(p1.get(&ColorSlot::Base05), p2.get(&ColorSlot::Base05));
    }

    #[test]
    fn test_polarity_dark() {
        let palette = build_palette(&base16_colors(), Scheme::Base16);
        assert_eq!(polarity(&palette), Some(Polarity::Dark));
    }

    #[test]
    fn test_polarity_light() {
        let mut colors = base16_colors();
        colors.insert("base00".into(), "#eff1f5".into());
        let palette = build_palette(&colors, Scheme::Base16);
        assert_eq!(polarity(&palette), Some(Polarity::Light));
    }

    #[test]
    fn test_validate_good_palette() {
        let palette = build_palette(&base16_colors(), Scheme::Base16);
        let failures = validate(&palette);
        assert!(failures.is_empty(), "unexpected failures: {:?}", failures);
    }

    #[test]
    fn test_validate_bad_contrast() {
        let mut colors = base16_colors();
        colors.insert("base05".into(), "#1f1f2f".into()); // fg ≈ bg
        let palette = build_palette(&colors, Scheme::Base16);
        let failures = validate(&palette);
        assert!(!failures.is_empty());
    }

    #[test]
    fn test_empty_palette() {
        let palette = build_palette(&HashMap::new(), Scheme::Base16);
        assert!(palette.is_empty());
    }
}

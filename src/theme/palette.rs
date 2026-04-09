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
use praxis::category::Entity;
use praxis_domains::science::colors::Rgb;
use praxis_domains::technology::theming::base16::{ColorSlot, Polarity};
use praxis_domains::technology::theming::ontology::{self, Palette};
use praxis_domains::technology::theming::schemes::{Ansi16Color, Vogix16Semantic};
use std::collections::HashMap;

/// Try to insert a color into the palette from a hex string.
#[allow(dead_code)]
fn insert_hex(palette: &mut Palette, slot: ColorSlot, hex: &str) {
    if let Some(rgb) = Rgb::from_hex(hex) {
        palette.insert(slot, rgb);
    }
}

/// Build a praxis Palette from raw theme colors.
///
/// Handles all scheme naming conventions using the praxis ontology
/// to map keys to ColorSlots.
#[allow(dead_code)]
pub fn build_palette(colors: &HashMap<String, String>, scheme: Scheme) -> Palette {
    let mut palette = Palette::new();

    match scheme {
        Scheme::Base16 | Scheme::Base24 => {
            for slot in ColorSlot::variants() {
                if let Some(hex) = colors.get(slot.key()) {
                    insert_hex(&mut palette, slot, hex);
                }
            }
        }
        Scheme::Vogix16 => {
            for semantic in Vogix16Semantic::variants() {
                if let Some(hex) = colors.get(semantic.key()) {
                    insert_hex(&mut palette, semantic.to_slot(), hex);
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
#[allow(dead_code)]
pub fn polarity(palette: &Palette) -> Option<Polarity> {
    ontology::detect_polarity(palette)
}

/// Validate palette against praxis axioms.
/// Returns list of failed axiom descriptions, empty if all pass.
#[allow(dead_code)]
pub fn validate(palette: &Palette) -> Vec<String> {
    use praxis::ontology::Axiom;
    use praxis_domains::technology::theming::ontology::{
        LuminanceMonotonicity, WcagForegroundContrast,
    };

    let mut failures = Vec::new();

    let mono = LuminanceMonotonicity {
        palette: palette.clone(),
    };
    if !mono.holds() {
        failures.push(mono.description().to_string());
    }

    let contrast = WcagForegroundContrast {
        palette: palette.clone(),
    };
    if !contrast.holds() {
        failures.push(contrast.description().to_string());
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

    fn vogix16_colors() -> HashMap<String, String> {
        let mut c = HashMap::new();
        c.insert("background".into(), "#1e1e2e".into());
        c.insert("background-surface".into(), "#313244".into());
        c.insert("background-selection".into(), "#45475a".into());
        c.insert("foreground-comment".into(), "#585b70".into());
        c.insert("foreground-border".into(), "#6c7086".into());
        c.insert("foreground-text".into(), "#cdd6f4".into());
        c.insert("foreground-heading".into(), "#d8dee8".into());
        c.insert("foreground-bright".into(), "#eceff4".into());
        c.insert("success".into(), "#f38ba8".into());
        c.insert("warning".into(), "#fab387".into());
        c.insert("notice".into(), "#f9e2af".into());
        c.insert("danger".into(), "#a6e3a1".into());
        c.insert("active".into(), "#94e2d5".into());
        c.insert("link".into(), "#89b4fa".into());
        c.insert("highlight".into(), "#cba6f7".into());
        c.insert("special".into(), "#f2cdcd".into());
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

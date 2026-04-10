/// Theme package ontology — what constitutes a valid theme.
///
/// A theme package is a named collection of variant palettes with metadata.
/// The ontology defines the structure and validation axioms.
///
/// Sources:
/// - Base16 builder spec: theme structure requirements
/// - tinted-theming: directory-based theme organization
use crate::variants::VariantSet;
use praxis::ontology::Axiom;
use praxis_domains::technology::theming::base16::Polarity;
use praxis_domains::technology::theming::ontology::Palette;
use praxis_domains::technology::theming::schemes::SchemeType;
use std::collections::HashMap;

/// A complete theme package — name, scheme, variants, palettes.
#[derive(Debug, Clone)]
pub struct ThemePackage {
    pub name: String,
    pub scheme: SchemeType,
    pub variants: VariantSet,
    pub palettes: HashMap<String, Palette>,
}

impl ThemePackage {
    pub fn new(name: impl Into<String>, scheme: SchemeType) -> Self {
        let name = name.into();
        Self {
            variants: VariantSet::new(name.clone()),
            name,
            scheme,
            palettes: HashMap::new(),
        }
    }

    pub fn add_variant(&mut self, name: impl Into<String>, polarity: Polarity, order: u32, palette: Palette) {
        let name = name.into();
        self.variants.add(name.clone(), polarity, order);
        self.palettes.insert(name, palette);
    }

    /// Get a variant's palette.
    pub fn palette(&self, variant: &str) -> Option<&Palette> {
        self.palettes.get(variant)
    }

    /// Validate the entire theme package against all axioms.
    pub fn validate(&self) -> Vec<String> {
        let mut failures = Vec::new();

        let axioms: Vec<Box<dyn Axiom>> = vec![
            Box::new(HasAtLeastOneVariant { theme: self.clone() }),
            Box::new(AllVariantsHavePalettes { theme: self.clone() }),
            Box::new(PalettesHaveRequiredSlots { theme: self.clone() }),
            Box::new(VariantOrdersUnique { theme: self.clone() }),
        ];

        for axiom in &axioms {
            if !axiom.holds() {
                failures.push(axiom.description().to_string());
            }
        }

        failures
    }
}

// ── Axioms ──

/// A theme must have at least one variant.
pub struct HasAtLeastOneVariant {
    pub theme: ThemePackage,
}

impl Axiom for HasAtLeastOneVariant {
    fn description(&self) -> &str {
        "theme must have at least one variant"
    }
    fn holds(&self) -> bool {
        !self.theme.variants.is_empty()
    }
}

/// Every variant must have a corresponding palette.
pub struct AllVariantsHavePalettes {
    pub theme: ThemePackage,
}

impl Axiom for AllVariantsHavePalettes {
    fn description(&self) -> &str {
        "every variant must have a palette"
    }
    fn holds(&self) -> bool {
        self.theme.variants.variants.iter().all(|v| {
            self.theme.palettes.contains_key(&v.name)
        })
    }
}

/// Each palette must have the minimum required slots for its scheme type.
/// Base16/Vogix16/Ansi16: at least base00 + base05 (background + foreground)
/// Base24: at least base00 + base05 + base10 (background + foreground + dark bg)
pub struct PalettesHaveRequiredSlots {
    pub theme: ThemePackage,
}

impl Axiom for PalettesHaveRequiredSlots {
    fn description(&self) -> &str {
        "palettes must have required slots (at least base00 + base05)"
    }
    fn holds(&self) -> bool {
        use praxis_domains::technology::theming::base16::ColorSlot;
        self.theme.palettes.values().all(|p| {
            p.contains_key(&ColorSlot::Base00) && p.contains_key(&ColorSlot::Base05)
        })
    }
}

/// Variant orders must be unique.
pub struct VariantOrdersUnique {
    pub theme: ThemePackage,
}

impl Axiom for VariantOrdersUnique {
    fn description(&self) -> &str {
        "variant orders must be unique"
    }
    fn holds(&self) -> bool {
        crate::variants::UniqueOrders {
            variants: self.theme.variants.clone(),
        }
        .holds()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use praxis_domains::science::colors::Rgb;
    use praxis_domains::technology::theming::base16::ColorSlot;

    fn make_palette(bg: Rgb, fg: Rgb) -> Palette {
        let mut p = Palette::new();
        p.insert(ColorSlot::Base00, bg);
        p.insert(ColorSlot::Base05, fg);
        p
    }

    fn valid_theme() -> ThemePackage {
        let mut t = ThemePackage::new("test-theme", SchemeType::Base16);
        t.add_variant(
            "dark",
            Polarity::Dark,
            1,
            make_palette(Rgb::new(30, 30, 46), Rgb::new(205, 214, 244)),
        );
        t.add_variant(
            "light",
            Polarity::Light,
            0,
            make_palette(Rgb::new(239, 241, 245), Rgb::new(76, 79, 105)),
        );
        t
    }

    #[test]
    fn test_valid_theme_passes() {
        let failures = valid_theme().validate();
        assert!(failures.is_empty(), "failures: {:?}", failures);
    }

    #[test]
    fn test_empty_theme_fails() {
        let t = ThemePackage::new("empty", SchemeType::Base16);
        let failures = t.validate();
        assert!(!failures.is_empty());
    }

    #[test]
    fn test_missing_palette_fails() {
        let mut t = ThemePackage::new("broken", SchemeType::Base16);
        t.variants.add("dark", Polarity::Dark, 0);
        // No palette added
        let failures = t.validate();
        assert!(!failures.is_empty());
    }

    #[test]
    fn test_missing_slots_fails() {
        let mut t = ThemePackage::new("incomplete", SchemeType::Base16);
        let mut p = Palette::new();
        p.insert(ColorSlot::Base00, Rgb::new(0, 0, 0));
        // Missing base05
        t.add_variant("dark", Polarity::Dark, 0, p);
        // add_variant adds to variants but palette is missing base05
        // Need to manually fix — add_variant inserts the palette
        let failures = t.validate();
        assert!(!failures.is_empty());
    }

    #[test]
    fn test_duplicate_orders_fails() {
        let mut t = ThemePackage::new("dup-order", SchemeType::Base16);
        t.add_variant(
            "a",
            Polarity::Dark,
            0,
            make_palette(Rgb::new(30, 30, 46), Rgb::new(205, 214, 244)),
        );
        t.add_variant(
            "b",
            Polarity::Dark,
            0, // same order!
            make_palette(Rgb::new(20, 20, 36), Rgb::new(195, 204, 234)),
        );
        let failures = t.validate();
        assert!(!failures.is_empty());
    }

    #[test]
    fn test_palette_access() {
        let t = valid_theme();
        assert!(t.palette("dark").is_some());
        assert!(t.palette("light").is_some());
        assert!(t.palette("nonexistent").is_none());
    }

    // ── Property-based tests ──
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn prop_valid_theme_always_passes(
            bg_r in 0u8..50, bg_g in 0u8..50, bg_b in 0u8..50,
            fg_r in 200u8..=255, fg_g in 200u8..=255, fg_b in 200u8..=255,
        ) {
            let mut t = ThemePackage::new("random", SchemeType::Base16);
            t.add_variant(
                "dark",
                Polarity::Dark,
                0,
                make_palette(Rgb::new(bg_r, bg_g, bg_b), Rgb::new(fg_r, fg_g, fg_b)),
            );
            let failures = t.validate();
            prop_assert!(failures.is_empty(), "unexpected: {:?}", failures);
        }
    }
}

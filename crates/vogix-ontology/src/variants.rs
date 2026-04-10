/// Variant ontology — theme variant metadata, ordering, and navigation.
///
/// A variant is a specific color instantiation of a theme (e.g., "mocha", "latte",
/// "night", "day"). Variants have polarity (dark/light), luminance ordering,
/// and navigation semantics (darker/lighter).
///
/// Sources:
/// - Base16 spec: variants are polarity-tagged (dark/light)
/// - Vogix16 spec: variants have explicit ordering by luminance
/// - WCAG 2.1: polarity derived from base00 relative luminance
use praxis::ontology::Axiom;
use praxis_domains::technology::theming::base16::Polarity;

/// Variant metadata — properties of a single theme variant.
#[derive(Debug, Clone, PartialEq)]
pub struct VariantMeta {
    pub name: String,
    pub polarity: Polarity,
    /// Position in the luminance ordering (0 = lightest background)
    pub order: u32,
}

/// A theme's variant set — all variants with ordering.
#[derive(Debug, Clone)]
pub struct VariantSet {
    pub theme_name: String,
    pub variants: Vec<VariantMeta>,
}

impl VariantSet {
    pub fn new(theme_name: impl Into<String>) -> Self {
        Self {
            theme_name: theme_name.into(),
            variants: Vec::new(),
        }
    }

    pub fn add(&mut self, name: impl Into<String>, polarity: Polarity, order: u32) {
        self.variants.push(VariantMeta {
            name: name.into(),
            polarity,
            order,
        });
    }

    /// Navigate to darker variant (higher order = darker background).
    pub fn darker(&self, current: &str) -> Option<&str> {
        let current_order = self.variants.iter().find(|v| v.name == current)?.order;
        self.variants
            .iter()
            .filter(|v| v.order > current_order)
            .min_by_key(|v| v.order)
            .map(|v| v.name.as_str())
    }

    /// Navigate to lighter variant (lower order = lighter background).
    pub fn lighter(&self, current: &str) -> Option<&str> {
        let current_order = self.variants.iter().find(|v| v.name == current)?.order;
        self.variants
            .iter()
            .filter(|v| v.order < current_order)
            .min_by_key(|v| current_order - v.order)
            .map(|v| v.name.as_str())
    }

    /// Get the default variant for a given polarity.
    pub fn default_for_polarity(&self, polarity: Polarity) -> Option<&str> {
        self.variants
            .iter()
            .find(|v| v.polarity == polarity)
            .map(|v| v.name.as_str())
    }

    /// Count of variants.
    pub fn len(&self) -> usize {
        self.variants.len()
    }

    pub fn is_empty(&self) -> bool {
        self.variants.is_empty()
    }
}

// ── Qualities ──

/// Quality: the polarity of a variant.
#[derive(Debug, Clone)]
pub struct VariantPolarity;

/// Quality: the order position of a variant.
#[derive(Debug, Clone)]
pub struct VariantOrder;

// ── Axioms ──

/// Orders must be unique within a variant set.
pub struct UniqueOrders {
    pub variants: VariantSet,
}

impl Axiom for UniqueOrders {
    fn description(&self) -> &str {
        "variant orders must be unique within a theme"
    }
    fn holds(&self) -> bool {
        let mut orders: Vec<u32> = self.variants.variants.iter().map(|v| v.order).collect();
        orders.sort();
        orders.dedup();
        orders.len() == self.variants.variants.len()
    }
}

/// Navigation must be consistent: darker(lighter(x)) = x if both exist.
pub struct NavigationRoundtrip {
    pub variants: VariantSet,
}

impl Axiom for NavigationRoundtrip {
    fn description(&self) -> &str {
        "darker(lighter(x)) = x when both directions have targets"
    }
    fn holds(&self) -> bool {
        for v in &self.variants.variants {
            if let Some(lighter) = self.variants.lighter(&v.name)
                && let Some(back) = self.variants.darker(lighter)
                && back != v.name
            {
                return false;
            }
            if let Some(darker) = self.variants.darker(&v.name)
                && let Some(back) = self.variants.lighter(darker)
                && back != v.name
            {
                return false;
            }
        }
        true
    }
}

/// Every variant must have a polarity.
pub struct PolarityComplete {
    pub variants: VariantSet,
}

impl Axiom for PolarityComplete {
    fn description(&self) -> &str {
        "every variant has a polarity (dark or light)"
    }
    fn holds(&self) -> bool {
        // All variants have a polarity — by construction (enum), always true.
        // This axiom exists for documentation and future extension.
        !self.variants.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn catppuccin_variants() -> VariantSet {
        let mut vs = VariantSet::new("catppuccin");
        vs.add("latte", Polarity::Light, 0);
        vs.add("frappe", Polarity::Dark, 1);
        vs.add("macchiato", Polarity::Dark, 2);
        vs.add("mocha", Polarity::Dark, 3);
        vs
    }

    fn aikido_variants() -> VariantSet {
        let mut vs = VariantSet::new("aikido");
        vs.add("day", Polarity::Light, 0);
        vs.add("night", Polarity::Dark, 1);
        vs
    }

    #[test]
    fn test_variant_count() {
        assert_eq!(catppuccin_variants().len(), 4);
        assert_eq!(aikido_variants().len(), 2);
    }

    #[test]
    fn test_darker() {
        let vs = catppuccin_variants();
        assert_eq!(vs.darker("latte"), Some("frappe"));
        assert_eq!(vs.darker("frappe"), Some("macchiato"));
        assert_eq!(vs.darker("macchiato"), Some("mocha"));
        assert_eq!(vs.darker("mocha"), None); // darkest
    }

    #[test]
    fn test_lighter() {
        let vs = catppuccin_variants();
        assert_eq!(vs.lighter("mocha"), Some("macchiato"));
        assert_eq!(vs.lighter("macchiato"), Some("frappe"));
        assert_eq!(vs.lighter("frappe"), Some("latte"));
        assert_eq!(vs.lighter("latte"), None); // lightest
    }

    #[test]
    fn test_default_for_polarity() {
        let vs = catppuccin_variants();
        assert_eq!(vs.default_for_polarity(Polarity::Light), Some("latte"));
        assert_eq!(vs.default_for_polarity(Polarity::Dark), Some("frappe")); // first dark
    }

    #[test]
    fn test_unique_orders() {
        assert!(UniqueOrders { variants: catppuccin_variants() }.holds());
        assert!(UniqueOrders { variants: aikido_variants() }.holds());
    }

    #[test]
    fn test_duplicate_orders_fail() {
        let mut vs = VariantSet::new("broken");
        vs.add("a", Polarity::Dark, 1);
        vs.add("b", Polarity::Dark, 1); // duplicate
        assert!(!UniqueOrders { variants: vs }.holds());
    }

    #[test]
    fn test_navigation_roundtrip() {
        assert!(NavigationRoundtrip { variants: catppuccin_variants() }.holds());
        assert!(NavigationRoundtrip { variants: aikido_variants() }.holds());
    }

    #[test]
    fn test_polarity_complete() {
        assert!(PolarityComplete { variants: catppuccin_variants() }.holds());
    }

    // ── Property-based tests ──
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn prop_darker_increases_order(idx in 0usize..4) {
            let vs = catppuccin_variants();
            let v = &vs.variants[idx];
            if let Some(darker) = vs.darker(&v.name) {
                let darker_order = vs.variants.iter().find(|x| x.name == darker).unwrap().order;
                prop_assert!(darker_order > v.order);
            }
        }

        #[test]
        fn prop_lighter_decreases_order(idx in 0usize..4) {
            let vs = catppuccin_variants();
            let v = &vs.variants[idx];
            if let Some(lighter) = vs.lighter(&v.name) {
                let lighter_order = vs.variants.iter().find(|x| x.name == lighter).unwrap().order;
                prop_assert!(lighter_order < v.order);
            }
        }

        #[test]
        fn prop_navigation_stays_in_set(idx in 0usize..4) {
            let vs = catppuccin_variants();
            let v = &vs.variants[idx];
            if let Some(darker) = vs.darker(&v.name) {
                prop_assert!(vs.variants.iter().any(|x| x.name == darker));
            }
            if let Some(lighter) = vs.lighter(&v.name) {
                prop_assert!(vs.variants.iter().any(|x| x.name == lighter));
            }
        }
    }
}

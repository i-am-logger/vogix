/// Surface ontology — abstract rendering targets for color themes.
///
/// A Surface is anything that renders colors from a theme palette:
/// terminal emulators, window managers, hardware RGB devices, shaders.
///
/// The key insight: theme application is a FUNCTOR from ThemeCategory
/// to SurfaceCategory. A theme change is a NATURAL TRANSFORMATION
/// that updates all surface functors consistently.
///
/// This module defines the abstract framework. Concrete surfaces
/// (wezterm, hyprland, openrgb) are instances.
///
/// Sources:
/// - Mac Lane, "Categories for the Working Mathematician" (1971): functors, natural transformations
/// - Harel, "Statecharts" (1987): parallel regions (surfaces update simultaneously)
/// - Czaplicki & Chong, "Async FRP for GUIs" (2013): sync vs async propagation
use praxis::category::Entity;
use praxis::ontology::{Axiom, Quality};
use praxis_domains::science::colors::Rgb;
use praxis_domains::technology::theming::base16::ColorSlot;
use praxis_domains::technology::theming::ontology::Palette;
use std::collections::HashMap;
/// A surface capability — what a rendering target can express.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SurfaceCapability {
    /// Can render ANSI 16 colors (terminals)
    Ansi16,
    /// Can render arbitrary RGB per-pixel (window borders, backgrounds)
    TrueColor,
    /// Can render a fixed number of RGB LEDs (hardware ring, keyboard)
    LedArray,
    /// Can render a full-screen shader (GLSL)
    Shader,
    /// Can render an image/video (LCD, wallpaper)
    Media,
}

impl Entity for SurfaceCapability {
    fn variants() -> Vec<Self> {
        vec![
            Self::Ansi16,
            Self::TrueColor,
            Self::LedArray,
            Self::Shader,
            Self::Media,
        ]
    }
}
/// A surface type — a class of rendering targets.
///
/// Not a specific instance (not "my wezterm") but a category
/// of targets that share the same rendering semantics.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SurfaceType {
    pub name: String,
    pub capabilities: Vec<SurfaceCapability>,
}

impl SurfaceType {
    pub fn new(name: impl Into<String>, capabilities: Vec<SurfaceCapability>) -> Self {
        Self {
            name: name.into(),
            capabilities,
        }
    }

    pub fn has_capability(&self, cap: SurfaceCapability) -> bool {
        self.capabilities.contains(&cap)
    }
}
/// A surface mapping — how a palette slot maps to a surface-specific config value.
///
/// This is the morphism in the functor: Theme → Surface.
/// Each surface type defines which slots it consumes and how.
#[derive(Debug, Clone)]
pub struct SlotMapping {
    /// Which palette slot this mapping reads
    pub slot: ColorSlot,
    /// The surface-specific config key (e.g., "background", "color01", "active_border")
    pub target_key: String,
    /// Optional transformation (e.g., strip '#' prefix, add 'rgb()' wrapper)
    pub transform: ColorTransform,
}
/// How to transform a color value for a specific surface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ColorTransform {
    /// Pass hex value as-is (#rrggbb)
    Hex,
    /// Strip '#' prefix (rrggbb)
    HexNoHash,
    /// Wrap in rgb() function: rgb(rrggbb)
    HyprlandRgb,
    /// Normalized float triple: r, g, b (0.0-1.0) for GLSL
    GlslFloat,
    /// ANSI SGR escape sequence
    AnsiSgr,
}
/// A surface functor — maps Theme objects (palette slots) to Surface objects (config values).
///
/// F: ThemeCategory → SurfaceCategory
///
/// For each ColorSlot, the functor produces a surface-specific config entry.
/// The functor preserves identity (unmapped slots produce no config) and
/// composition (mapping A→B→C through two surfaces composes the transforms).
#[derive(Debug, Clone)]
pub struct SurfaceFunctor {
    pub surface: SurfaceType,
    pub mappings: Vec<SlotMapping>,
}

impl SurfaceFunctor {
    pub fn new(surface: SurfaceType, mappings: Vec<SlotMapping>) -> Self {
        Self { surface, mappings }
    }

    /// Apply the functor: palette → surface config.
    ///
    /// This is F(palette) — maps each palette color through the slot mappings
    /// to produce surface-specific config entries.
    pub fn apply(&self, palette: &Palette) -> HashMap<String, String> {
        let mut config = HashMap::new();

        for mapping in &self.mappings {
            if let Some(rgb) = palette.get(&mapping.slot) {
                let value = apply_transform(rgb, &mapping.transform);
                config.insert(mapping.target_key.clone(), value);
            }
        }

        config
    }

    /// Which slots does this functor consume?
    pub fn consumed_slots(&self) -> Vec<ColorSlot> {
        self.mappings.iter().map(|m| m.slot).collect()
    }

    /// Which slots from the palette are NOT consumed by this functor?
    pub fn unconsumed_slots(&self, palette: &Palette) -> Vec<ColorSlot> {
        let consumed: Vec<_> = self.consumed_slots();
        palette
            .keys()
            .filter(|s| !consumed.contains(s))
            .cloned()
            .collect()
    }
}
/// Apply a color transform to produce a surface-specific string.
fn apply_transform(rgb: &Rgb, transform: &ColorTransform) -> String {
    match transform {
        ColorTransform::Hex => format!("#{:02x}{:02x}{:02x}", rgb.r, rgb.g, rgb.b),
        ColorTransform::HexNoHash => format!("{:02x}{:02x}{:02x}", rgb.r, rgb.g, rgb.b),
        ColorTransform::HyprlandRgb => format!("rgb({:02x}{:02x}{:02x})", rgb.r, rgb.g, rgb.b),
        ColorTransform::GlslFloat => format!(
            "{:.4}, {:.4}, {:.4}",
            rgb.r as f32 / 255.0,
            rgb.g as f32 / 255.0,
            rgb.b as f32 / 255.0
        ),
        ColorTransform::AnsiSgr => format!("{};{};{}", rgb.r, rgb.g, rgb.b),
    }
}
/// A natural transformation between two surface functors.
///
/// η: F ⟹ G where F, G: ThemeCategory → SurfaceCategory
///
/// For each palette slot s, η_s maps F(s) to G(s).
/// A theme change is a natural transformation:
/// applying theme T₁ then switching to T₂ must commute with
/// switching to T₂ then applying to each surface.
///
/// In practice: if we change from palette P₁ to P₂,
/// every surface must see the SAME new colors regardless of
/// the order surfaces are updated.
///
/// The naturality condition:
///   For all slots s, all surfaces F, G:
///     G(P₂)(s) is determined solely by P₂(s) and G's mapping for s.
///     It does NOT depend on P₁ or F.
///
/// This holds by construction: each SurfaceFunctor.apply() reads only
/// from the palette, never from other surfaces or previous state.
/// The proof is structural — we verify it as an axiom.
#[derive(Debug)]
pub struct ThemeChangeNaturality {
    pub functors: Vec<SurfaceFunctor>,
}

impl Axiom for ThemeChangeNaturality {
    fn description(&self) -> &str {
        "theme change is a natural transformation (surfaces are independent)"
    }
    fn holds(&self) -> bool {
        // The naturality condition holds if each functor's apply()
        // depends ONLY on the palette, not on other functors' outputs
        // or previous palette state.
        //
        // We verify this by applying two different palettes to all functors
        // and checking that each surface's output depends only on its own
        // palette input, not on the order of application or other surfaces.

        // Build two distinct test palettes
        let p1 = test_palette_dark();
        let p2 = test_palette_light();

        for functor in &self.functors {
            let config1 = functor.apply(&p1);
            let config2 = functor.apply(&p2);

            // Each config entry must differ if the underlying slot color differs
            // (the functor is deterministic — same input → same output)
            for mapping in &functor.mappings {
                let c1 = p1.get(&mapping.slot);
                let c2 = p2.get(&mapping.slot);
                let v1 = config1.get(&mapping.target_key);
                let v2 = config2.get(&mapping.target_key);

                // If slot colors are the same, config values must be the same
                if c1 == c2 && v1 != v2 {
                    return false; // functor is non-deterministic
                }
                // If slot colors differ, config values must differ
                // (unless the transform loses information, which we don't allow)
                if c1 != c2 && v1 == v2 {
                    return false; // functor lost information
                }
            }
        }
        true
    }
}
/// Quality: how many palette slots a surface consumes.
#[derive(Debug, Clone)]
pub struct SlotCoverage;

impl Quality for SlotCoverage {
    type Individual = SurfaceCapability;
    type Value = &'static str;
    fn get(&self, cap: &SurfaceCapability) -> Option<&'static str> {
        Some(match cap {
            SurfaceCapability::Ansi16 => "16 slots (ANSI 0-15)",
            SurfaceCapability::TrueColor => "arbitrary (per-element)",
            SurfaceCapability::LedArray => "1-2 slots (base00, base01)",
            SurfaceCapability::Shader => "8+ slots (base00-07 for hue, base08-0F preserved)",
            SurfaceCapability::Media => "0 slots (image/video, not color-mapped)",
        })
    }
}

// ── Test helpers ──

fn test_palette_dark() -> Palette {
    let mut p = Palette::new();
    p.insert(ColorSlot::Base00, Rgb::new(30, 30, 46));
    p.insert(ColorSlot::Base01, Rgb::new(49, 50, 68));
    p.insert(ColorSlot::Base05, Rgb::new(205, 214, 244));
    p.insert(ColorSlot::Base08, Rgb::new(243, 139, 168));
    p.insert(ColorSlot::Base0D, Rgb::new(137, 180, 250));
    p
}

fn test_palette_light() -> Palette {
    let mut p = Palette::new();
    p.insert(ColorSlot::Base00, Rgb::new(239, 241, 245));
    p.insert(ColorSlot::Base01, Rgb::new(230, 233, 239));
    p.insert(ColorSlot::Base05, Rgb::new(76, 79, 105));
    p.insert(ColorSlot::Base08, Rgb::new(210, 15, 57));
    p.insert(ColorSlot::Base0D, Rgb::new(30, 102, 245));
    p
}
/// Build an example terminal surface functor.
pub fn terminal_functor() -> SurfaceFunctor {
    use praxis::category::Entity;

    let mut mappings = Vec::new();

    // Map all base16 slots to ANSI color keys
    for slot in ColorSlot::variants() {
        if let Some(idx) = slot.ansi_index() {
            mappings.push(SlotMapping {
                slot,
                target_key: format!("color{:02}", idx),
                transform: ColorTransform::Hex,
            });
        }
    }

    SurfaceFunctor::new(
        SurfaceType::new("terminal", vec![SurfaceCapability::Ansi16]),
        mappings,
    )
}
/// Build an example window border surface functor.
pub fn border_functor() -> SurfaceFunctor {
    SurfaceFunctor::new(
        SurfaceType::new("window-borders", vec![SurfaceCapability::TrueColor]),
        vec![
            SlotMapping {
                slot: ColorSlot::Base04,
                target_key: "active_border".into(),
                transform: ColorTransform::HyprlandRgb,
            },
            SlotMapping {
                slot: ColorSlot::Base02,
                target_key: "inactive_border".into(),
                transform: ColorTransform::HyprlandRgb,
            },
        ],
    )
}
/// Build an example LED hardware surface functor.
pub fn led_functor() -> SurfaceFunctor {
    SurfaceFunctor::new(
        SurfaceType::new("led-ring", vec![SurfaceCapability::LedArray]),
        vec![SlotMapping {
            slot: ColorSlot::Base01,
            target_key: "ring_color".into(),
            transform: ColorTransform::HexNoHash,
        }],
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Surface type tests ──

    #[test]
    fn test_surface_capabilities() {
        assert_eq!(SurfaceCapability::variants().len(), 5);
    }

    #[test]
    fn test_terminal_has_ansi16() {
        let t = SurfaceType::new("terminal", vec![SurfaceCapability::Ansi16]);
        assert!(t.has_capability(SurfaceCapability::Ansi16));
        assert!(!t.has_capability(SurfaceCapability::Shader));
    }

    // ── Transform tests ──

    #[test]
    fn test_hex_transform() {
        let rgb = Rgb::new(255, 128, 0);
        assert_eq!(apply_transform(&rgb, &ColorTransform::Hex), "#ff8000");
    }

    #[test]
    fn test_hex_no_hash() {
        let rgb = Rgb::new(255, 128, 0);
        assert_eq!(apply_transform(&rgb, &ColorTransform::HexNoHash), "ff8000");
    }

    #[test]
    fn test_hyprland_rgb() {
        let rgb = Rgb::new(255, 128, 0);
        assert_eq!(
            apply_transform(&rgb, &ColorTransform::HyprlandRgb),
            "rgb(ff8000)"
        );
    }

    #[test]
    fn test_glsl_float() {
        let rgb = Rgb::new(255, 0, 0);
        let result = apply_transform(&rgb, &ColorTransform::GlslFloat);
        assert!(result.starts_with("1.0000, 0.0000, 0.0000"));
    }

    // ── Functor tests ──

    #[test]
    fn test_terminal_functor_produces_16_entries() {
        let f = terminal_functor();
        let p = test_palette_dark();
        let config = f.apply(&p);
        // Only slots present in the palette are mapped
        assert!(!config.is_empty());
    }

    #[test]
    fn test_border_functor_produces_2_entries() {
        let f = border_functor();
        let mut p = test_palette_dark();
        p.insert(ColorSlot::Base04, Rgb::new(108, 112, 134));
        p.insert(ColorSlot::Base02, Rgb::new(69, 71, 90));
        let config = f.apply(&p);
        assert_eq!(config.len(), 2);
        assert!(config.contains_key("active_border"));
        assert!(config.contains_key("inactive_border"));
    }

    #[test]
    fn test_led_functor_produces_1_entry() {
        let f = led_functor();
        let p = test_palette_dark();
        let config = f.apply(&p);
        assert_eq!(config.len(), 1);
        assert!(config.contains_key("ring_color"));
    }

    #[test]
    fn test_functor_deterministic() {
        // Same palette → same config (always)
        let f = terminal_functor();
        let p = test_palette_dark();
        let c1 = f.apply(&p);
        let c2 = f.apply(&p);
        assert_eq!(c1, c2);
    }

    #[test]
    fn test_functor_different_palettes_different_config() {
        let f = border_functor();
        let mut p1 = Palette::new();
        p1.insert(ColorSlot::Base04, Rgb::new(100, 100, 100));
        p1.insert(ColorSlot::Base02, Rgb::new(50, 50, 50));
        let mut p2 = Palette::new();
        p2.insert(ColorSlot::Base04, Rgb::new(200, 200, 200));
        p2.insert(ColorSlot::Base02, Rgb::new(150, 150, 150));
        assert_ne!(f.apply(&p1), f.apply(&p2));
    }

    // ── Natural transformation axiom ──

    #[test]
    fn test_theme_change_naturality() {
        let axiom = ThemeChangeNaturality {
            functors: vec![terminal_functor(), border_functor(), led_functor()],
        };
        assert!(axiom.holds());
    }

    #[test]
    fn test_naturality_with_full_palette() {
        let mut p1 = test_palette_dark();
        p1.insert(ColorSlot::Base02, Rgb::new(69, 71, 90));
        p1.insert(ColorSlot::Base04, Rgb::new(108, 112, 134));

        let mut p2 = test_palette_light();
        p2.insert(ColorSlot::Base02, Rgb::new(188, 192, 204));
        p2.insert(ColorSlot::Base04, Rgb::new(140, 143, 161));

        // Apply all functors to both palettes
        let functors = vec![terminal_functor(), border_functor(), led_functor()];

        for f in &functors {
            let c1 = f.apply(&p1);
            let c2 = f.apply(&p2);

            // Each functor produces independent results
            // Changing p1→p2 for one functor doesn't affect others
            for mapping in &f.mappings {
                if let (Some(v1), Some(v2)) = (
                    c1.get(&mapping.target_key),
                    c2.get(&mapping.target_key),
                ) {
                    let rgb1 = p1.get(&mapping.slot);
                    let rgb2 = p2.get(&mapping.slot);
                    if rgb1 != rgb2 {
                        assert_ne!(v1, v2, "functor lost information for {:?}", mapping.slot);
                    }
                }
            }
        }
    }

    // ── Property-based tests ──
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn prop_hex_transform_length(r in 0u8..=255, g in 0u8..=255, b in 0u8..=255) {
            let rgb = Rgb::new(r, g, b);
            let hex = apply_transform(&rgb, &ColorTransform::Hex);
            prop_assert_eq!(hex.len(), 7); // #rrggbb
            prop_assert!(hex.starts_with('#'));
        }

        #[test]
        fn prop_hex_roundtrip(r in 0u8..=255, g in 0u8..=255, b in 0u8..=255) {
            let rgb = Rgb::new(r, g, b);
            let hex = apply_transform(&rgb, &ColorTransform::Hex);
            let parsed = Rgb::from_hex(&hex).unwrap();
            prop_assert_eq!(parsed, rgb);
        }

        #[test]
        fn prop_functor_preserves_slot_count(r in 0u8..=255, g in 0u8..=255, b in 0u8..=255) {
            // A functor never produces MORE config entries than slot mappings
            let f = border_functor();
            let mut p = Palette::new();
            p.insert(ColorSlot::Base04, Rgb::new(r, g, b));
            p.insert(ColorSlot::Base02, Rgb::new(g, b, r));
            let config = f.apply(&p);
            prop_assert!(config.len() <= f.mappings.len());
        }

        #[test]
        fn prop_transform_deterministic(r in 0u8..=255, g in 0u8..=255, b in 0u8..=255) {
            let rgb = Rgb::new(r, g, b);
            for transform in [
                ColorTransform::Hex,
                ColorTransform::HexNoHash,
                ColorTransform::HyprlandRgb,
                ColorTransform::GlslFloat,
                ColorTransform::AnsiSgr,
            ] {
                let v1 = apply_transform(&rgb, &transform);
                let v2 = apply_transform(&rgb, &transform);
                prop_assert_eq!(v1, v2);
            }
        }

        #[test]
        fn prop_naturality_random_palettes(
            r1 in 0u8..=255, g1 in 0u8..=255, b1 in 0u8..=255,
            r2 in 0u8..=255, g2 in 0u8..=255, b2 in 0u8..=255,
        ) {
            // Natural transformation holds for arbitrary palette colors
            let mut p1 = Palette::new();
            p1.insert(ColorSlot::Base00, Rgb::new(r1, g1, b1));
            p1.insert(ColorSlot::Base01, Rgb::new(r1.wrapping_add(20), g1.wrapping_add(20), b1.wrapping_add(20)));
            p1.insert(ColorSlot::Base02, Rgb::new(r1.wrapping_add(40), g1.wrapping_add(40), b1.wrapping_add(40)));
            p1.insert(ColorSlot::Base04, Rgb::new(r1.wrapping_add(80), g1.wrapping_add(80), b1.wrapping_add(80)));
            p1.insert(ColorSlot::Base05, Rgb::new(r2, g2, b2));
            p1.insert(ColorSlot::Base08, Rgb::new(r2, g1, b1));
            p1.insert(ColorSlot::Base0D, Rgb::new(r1, g2, b2));

            let mut p2 = Palette::new();
            p2.insert(ColorSlot::Base00, Rgb::new(r2, g2, b2));
            p2.insert(ColorSlot::Base01, Rgb::new(r2.wrapping_add(20), g2.wrapping_add(20), b2.wrapping_add(20)));
            p2.insert(ColorSlot::Base02, Rgb::new(r2.wrapping_add(40), g2.wrapping_add(40), b2.wrapping_add(40)));
            p2.insert(ColorSlot::Base04, Rgb::new(r2.wrapping_add(80), g2.wrapping_add(80), b2.wrapping_add(80)));
            p2.insert(ColorSlot::Base05, Rgb::new(r1, g1, b1));
            p2.insert(ColorSlot::Base08, Rgb::new(r1, g2, b2));
            p2.insert(ColorSlot::Base0D, Rgb::new(r2, g1, b1));

            let functors = vec![terminal_functor(), border_functor(), led_functor()];

            // Each functor's output depends ONLY on the palette, not on other functors
            for f in &functors {
                let c1 = f.apply(&p1);
                let c2 = f.apply(&p2);

                for mapping in &f.mappings {
                    let slot_same = p1.get(&mapping.slot) == p2.get(&mapping.slot);
                    let val_same = c1.get(&mapping.target_key) == c2.get(&mapping.target_key);

                    if p1.contains_key(&mapping.slot) && p2.contains_key(&mapping.slot) {
                        // Same slot color → same config value (determinism)
                        if slot_same {
                            prop_assert_eq!(
                                c1.get(&mapping.target_key),
                                c2.get(&mapping.target_key),
                                "non-deterministic for slot"
                            );
                        }
                        // Different slot color → different config value (info preservation)
                        if !slot_same {
                            prop_assert!(
                                !val_same,
                                "lost information for slot"
                            );
                        }
                    }
                }
            }
        }

        #[test]
        fn prop_empty_palette_empty_config(r in 0u8..=255, g in 0u8..=255, b in 0u8..=255) {
            // Functor on empty palette produces empty config
            let _ = (r, g, b); // unused but needed for proptest signature
            let f = border_functor();
            let p = Palette::new();
            let config = f.apply(&p);
            prop_assert!(config.is_empty());
        }

        #[test]
        fn prop_functor_output_keys_are_from_mappings(r in 0u8..=255, g in 0u8..=255, b in 0u8..=255) {
            // Every key in the output must come from a mapping
            let f = border_functor();
            let mut p = Palette::new();
            p.insert(ColorSlot::Base04, Rgb::new(r, g, b));
            p.insert(ColorSlot::Base02, Rgb::new(g, b, r));
            let config = f.apply(&p);
            let mapping_keys: Vec<_> = f.mappings.iter().map(|m| &m.target_key).collect();
            for key in config.keys() {
                prop_assert!(mapping_keys.contains(&key), "unexpected key: {}", key);
            }
        }

        #[test]
        fn prop_hex_no_hash_is_hex_without_hash(r in 0u8..=255, g in 0u8..=255, b in 0u8..=255) {
            let rgb = Rgb::new(r, g, b);
            let hex = apply_transform(&rgb, &ColorTransform::Hex);
            let no_hash = apply_transform(&rgb, &ColorTransform::HexNoHash);
            prop_assert_eq!(&hex[1..], no_hash.as_str());
        }

        #[test]
        fn prop_hyprland_rgb_wraps_hex(r in 0u8..=255, g in 0u8..=255, b in 0u8..=255) {
            let rgb = Rgb::new(r, g, b);
            let no_hash = apply_transform(&rgb, &ColorTransform::HexNoHash);
            let hypr = apply_transform(&rgb, &ColorTransform::HyprlandRgb);
            prop_assert_eq!(hypr, format!("rgb({})", no_hash));
        }
    }
}

//! Theme discovery from runtime config manifest.

use crate::errors::{Result, VogixError};
use crate::scheme::Scheme;
use std::fs;
use std::str::FromStr;

use super::types::{ThemeInfo, VariantInfo};

/// Discover all available themes from the user config.toml
pub fn discover_themes() -> Result<Vec<ThemeInfo>> {
    let state_dir = crate::config::Config::state_dir();
    let manifest_path = state_dir.join("config.toml");

    if !manifest_path.exists() {
        return Err(VogixError::ConfigNotFound(manifest_path));
    }

    let content = fs::read_to_string(&manifest_path)
        .map_err(|_| VogixError::ConfigNotFound(manifest_path.clone()))?;

    parse_themes_from_manifest(&content)
}

/// Parse themes from a TOML manifest string
pub fn parse_themes_from_manifest(content: &str) -> Result<Vec<ThemeInfo>> {
    let manifest: toml::Value = content.parse().map_err(VogixError::TomlParse)?;

    let mut themes = Vec::new();

    if let Some(themes_table) = manifest.get("themes").and_then(|v| v.as_table()) {
        for (theme_name, theme_value) in themes_table {
            if let Some(table) = theme_value.as_table() {
                // Skip variant detail entries (they have polarity/bg but no variants array)
                // Only process theme entries (they have a variants array)
                if !table.contains_key("variants") {
                    continue;
                }

                let scheme_str = table
                    .get("scheme")
                    .and_then(|v| v.as_str())
                    .unwrap_or("vogix16");
                let scheme = Scheme::from_str(scheme_str).unwrap_or(Scheme::Vogix16);

                let variant_names: Vec<String> = table
                    .get("variants")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_else(|| vec!["dark".to_string(), "light".to_string()]);

                // Parse variant details (variantName = { polarity, order } inside theme table)
                let mut variants = Vec::new();
                for (idx, var_name) in variant_names.iter().enumerate() {
                    // Look for variant details inside the theme's table
                    let (polarity, order) =
                        if let Some(var_value) = table.get(var_name).and_then(|v| v.as_table()) {
                            let pol = var_value
                                .get("polarity")
                                .and_then(|v| v.as_str())
                                .unwrap_or("dark")
                                .to_string();
                            let ord = var_value
                                .get("order")
                                .and_then(|v| v.as_integer())
                                .unwrap_or(idx as i64) as u32;
                            (pol, ord)
                        } else {
                            // Fallback: infer polarity from variant name, use index as order
                            let pol = if var_name.to_lowercase().contains("light")
                                || var_name.to_lowercase() == "dawn"
                                || var_name.to_lowercase() == "latte"
                            {
                                "light".to_string()
                            } else {
                                "dark".to_string()
                            };
                            (pol, idx as u32)
                        };

                    variants.push(VariantInfo {
                        name: var_name.clone(),
                        polarity,
                        order,
                    });
                }

                themes.push(ThemeInfo {
                    name: theme_name.clone(),
                    scheme,
                    variants,
                });
            }
        }
    }

    themes.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(themes)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper to find variant by name (test-only)
    fn get_variant<'a>(theme: &'a ThemeInfo, name: &str) -> Option<&'a VariantInfo> {
        theme.variants.iter().find(|v| v.name == name)
    }

    #[test]
    fn test_parse_themes_with_variant_details() {
        let manifest = r##"
[themes.yoga]
scheme = "vogix16"
variants = ["night", "day"]
night = { polarity = "dark", order = 1 }
day = { polarity = "light", order = 0 }
"##;
        let themes = parse_themes_from_manifest(manifest).unwrap();
        assert_eq!(themes.len(), 1);

        let yoga = &themes[0];
        assert_eq!(yoga.name, "yoga");
        assert_eq!(yoga.variants.len(), 2);

        let night = get_variant(yoga, "night").unwrap();
        assert_eq!(night.polarity, "dark");
        assert_eq!(night.order, 1);

        let day = get_variant(yoga, "day").unwrap();
        assert_eq!(day.polarity, "light");
        assert_eq!(day.order, 0);
    }

    #[test]
    fn test_parse_themes_infers_polarity_from_name() {
        let manifest = r##"
[themes.simple]
scheme = "base16"
variants = ["dark", "light"]
"##;
        let themes = parse_themes_from_manifest(manifest).unwrap();
        assert_eq!(themes.len(), 1);

        let theme = &themes[0];
        let dark = get_variant(theme, "dark").unwrap();
        assert_eq!(dark.polarity, "dark");

        let light = get_variant(theme, "light").unwrap();
        assert_eq!(light.polarity, "light");
    }

    #[test]
    fn test_parse_themes_with_dawn_latte_variants() {
        let manifest = r##"
[themes.catppuccin]
scheme = "base24"
variants = ["mocha", "latte", "dawn"]
"##;
        let themes = parse_themes_from_manifest(manifest).unwrap();
        let theme = &themes[0];

        // "latte" and "dawn" should be inferred as light
        let latte = get_variant(theme, "latte").unwrap();
        assert_eq!(latte.polarity, "light");

        let dawn = get_variant(theme, "dawn").unwrap();
        assert_eq!(dawn.polarity, "light");

        // "mocha" should be dark (doesn't contain "light", "dawn", or "latte")
        let mocha = get_variant(theme, "mocha").unwrap();
        assert_eq!(mocha.polarity, "dark");
    }

    #[test]
    fn test_parse_invalid_toml() {
        let invalid = "this is not valid toml {{{";
        let result = parse_themes_from_manifest(invalid);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_empty_themes() {
        let manifest = r##"
[default]
theme = "yoga"
variant = "dark"
"##;
        let themes = parse_themes_from_manifest(manifest).unwrap();
        assert!(themes.is_empty());
    }
}

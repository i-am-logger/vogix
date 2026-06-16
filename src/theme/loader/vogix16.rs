//! vogix16 theme color loader
//!
//! Loads colors from vogix16 TOML files and generates semantic color mappings.
//!
//! File format:
//! ```toml
//! polarity = "dark"
//! [colors]
//! base00 = "#262626"
//! base01 = "#333333"
//! ...
//! ```

use crate::errors::{Result, VogixError};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

/// Internal struct for parsing vogix16 TOML files.
/// Only `colors` is needed; serde ignores other fields in the TOML.
#[derive(Deserialize)]
struct Vogix16Theme {
    colors: HashMap<String, String>,
}

/// Load colors from a vogix16 theme file
///
/// Returns base colors plus semantic aliases (e.g., "background", "foreground_text").
pub fn load(content: &str, _path: &Path) -> Result<HashMap<String, String>> {
    use pr4xis::category::FinitelyGenerated;
    use pr4xis_domains::applied::hmi::theming::schemes::Vogix16Semantic;

    let theme: Vogix16Theme = toml::from_str(content).map_err(VogixError::TomlParse)?;

    // Start with the raw base16-shaped colors (base00..0F).
    let mut colors = theme.colors.clone();

    // Add vogix16 semantic aliases sourced from the praxis ontology — the single
    // definition of the slot↔role mapping (base08=success, base0B=danger, …) and
    // the snake_case key names — instead of a table duplicated here.
    for semantic in Vogix16Semantic::variants() {
        if let Some(v) = theme.colors.get(semantic.to_slot().key()) {
            colors.insert(semantic.key().to_string(), v.clone());
        }
    }

    Ok(colors)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_vogix16_colors() {
        let content = r##"polarity = "dark"

[colors]
base00 = "#262626"
base01 = "#333333"
base02 = "#3b3028"
base03 = "#54433a"
base04 = "#6c5d53"
base05 = "#a29990"
base06 = "#cbc3bc"
base07 = "#f6f5f0"
base08 = "#4d5645"
base09 = "#835538"
base0A = "#bfa46f"
base0B = "#d7503c"
base0C = "#8694a8"
base0D = "#658fbd"
base0E = "#896ea4"
base0F = "#7a5c42"
"##;

        let colors = load(content, Path::new("test.toml")).unwrap();

        // Check base colors exist
        assert_eq!(colors.get("base00"), Some(&"#262626".to_string()));
        assert_eq!(colors.get("base0F"), Some(&"#7a5c42".to_string()));

        // Check semantic mappings
        assert_eq!(colors.get("background"), Some(&"#262626".to_string()));
        assert_eq!(colors.get("foreground_text"), Some(&"#a29990".to_string()));
        assert_eq!(colors.get("danger"), Some(&"#d7503c".to_string()));
    }

    #[test]
    fn test_load_invalid_toml() {
        let result = load("not valid toml {{", Path::new("test.toml"));
        assert!(result.is_err());
    }

    #[test]
    fn test_derives_all_16_aliases_from_praxis() {
        use pr4xis::category::FinitelyGenerated;
        use pr4xis_domains::applied::hmi::theming::base16::ColorSlot;
        use pr4xis_domains::applied::hmi::theming::schemes::Vogix16Semantic;
        let body: String = ColorSlot::variants()
            .iter()
            .filter(|s| s.is_base16())
            .map(|s| format!("{} = \"#101010\"\n", s.key()))
            .collect();
        let colors = load(&format!("[colors]\n{body}"), Path::new("t.toml")).unwrap();
        for s in Vogix16Semantic::variants() {
            assert!(colors.contains_key(s.key()), "missing alias {}", s.key());
        }
    }
}

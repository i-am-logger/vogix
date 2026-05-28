//! Theme query functions.

use crate::scheme::Scheme;

use super::types::ThemeInfo;

/// Filter themes by scheme
pub fn filter_by_scheme(themes: &[ThemeInfo], scheme: &Scheme) -> Vec<ThemeInfo> {
    themes
        .iter()
        .filter(|t| &t.scheme == scheme)
        .cloned()
        .collect()
}

/// Get a specific theme by name
pub fn get_theme(themes: &[ThemeInfo], name: &str) -> Option<ThemeInfo> {
    themes.iter().find(|t| t.name == name).cloned()
}

#[cfg(test)]
mod tests {
    use super::super::types::VariantInfo;
    use super::*;

    fn sample_themes() -> Vec<ThemeInfo> {
        vec![
            ThemeInfo {
                name: "yoga".to_string(),
                scheme: Scheme::Vogix16,
                variants: vec![VariantInfo {
                    name: "night".to_string(),
                    polarity: "dark".to_string(),
                    order: 0,
                }],
            },
            ThemeInfo {
                name: "gruvbox".to_string(),
                scheme: Scheme::Base16,
                variants: vec![VariantInfo {
                    name: "dark".to_string(),
                    polarity: "dark".to_string(),
                    order: 0,
                }],
            },
            ThemeInfo {
                name: "solarized".to_string(),
                scheme: Scheme::Base16,
                variants: vec![
                    VariantInfo {
                        name: "light".to_string(),
                        polarity: "light".to_string(),
                        order: 0,
                    },
                    VariantInfo {
                        name: "dark".to_string(),
                        polarity: "dark".to_string(),
                        order: 1,
                    },
                ],
            },
        ]
    }

    #[test]
    fn test_filter_by_scheme() {
        let themes = sample_themes();

        let base16_themes = filter_by_scheme(&themes, &Scheme::Base16);
        assert_eq!(base16_themes.len(), 2);
        assert!(base16_themes.iter().any(|t| t.name == "gruvbox"));
        assert!(base16_themes.iter().any(|t| t.name == "solarized"));

        let vogix16_themes = filter_by_scheme(&themes, &Scheme::Vogix16);
        assert_eq!(vogix16_themes.len(), 1);
        assert_eq!(vogix16_themes[0].name, "yoga");

        let ansi16_themes = filter_by_scheme(&themes, &Scheme::Ansi16);
        assert!(ansi16_themes.is_empty());
    }

    #[test]
    fn test_get_theme() {
        let themes = sample_themes();

        let gruvbox = get_theme(&themes, "gruvbox");
        assert!(gruvbox.is_some());
        assert_eq!(gruvbox.unwrap().name, "gruvbox");

        let nonexistent = get_theme(&themes, "nonexistent");
        assert!(nonexistent.is_none());
    }

    #[test]
    fn test_get_theme_returns_clone() {
        let themes = sample_themes();
        let theme1 = get_theme(&themes, "yoga").unwrap();
        let theme2 = get_theme(&themes, "yoga").unwrap();

        // Both are independent clones
        assert_eq!(theme1.name, theme2.name);
    }
}

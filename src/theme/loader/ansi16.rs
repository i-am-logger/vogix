//! ansi16 theme color loader
//!
//! Loads colors from TOML files in Alacritty format with nested color sections.
//!
//! File format:
//! ```toml
//! [colors.primary]
//! background = '#282a36'
//! foreground = '#f8f8f2'
//!
//! [colors.cursor]
//! cursor = '#f8f8f2'
//! text = '#282a36'
//!
//! [colors.selection]
//! background = '#44475a'
//! text = '#ffffff'
//!
//! [colors.normal]
//! black = '#21222c'
//! red = '#ff5555'
//! green = '#50fa7b'
//! yellow = '#f1fa8c'
//! blue = '#bd93f9'
//! magenta = '#ff79c6'
//! cyan = '#8be9fd'
//! white = '#f8f8f2'
//!
//! [colors.bright]
//! black = '#6272a4'
//! red = '#ff6e6e'
//! ...
//! ```

use crate::errors::{Result, VogixError};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

#[derive(Deserialize)]
struct Ansi16Theme {
    colors: Ansi16Colors,
}

#[derive(Deserialize)]
struct Ansi16Colors {
    primary: Option<Ansi16Primary>,
    cursor: Option<Ansi16Cursor>,
    selection: Option<Ansi16Selection>,
    normal: Option<Ansi16Normal>,
    bright: Option<Ansi16Bright>,
}

#[derive(Deserialize)]
struct Ansi16Primary {
    background: Option<String>,
    foreground: Option<String>,
}

#[derive(Deserialize)]
struct Ansi16Cursor {
    cursor: Option<String>,
    text: Option<String>,
}

#[derive(Deserialize)]
struct Ansi16Selection {
    background: Option<String>,
    text: Option<String>,
}

#[derive(Deserialize)]
struct Ansi16Normal {
    black: Option<String>,
    red: Option<String>,
    green: Option<String>,
    yellow: Option<String>,
    blue: Option<String>,
    magenta: Option<String>,
    cyan: Option<String>,
    white: Option<String>,
}

#[derive(Deserialize)]
struct Ansi16Bright {
    black: Option<String>,
    red: Option<String>,
    green: Option<String>,
    yellow: Option<String>,
    blue: Option<String>,
    magenta: Option<String>,
    cyan: Option<String>,
    white: Option<String>,
}

/// Load colors from an ansi16 theme file
///
/// Returns colors mapped to standard names:
/// - primary: background, foreground
/// - cursor: cursor_bg, cursor_fg
/// - selection: selection_bg, selection_fg
/// - normal: color00-color07
/// - bright: color08-color15
pub fn load(content: &str, _path: &Path) -> Result<HashMap<String, String>> {
    let theme: Ansi16Theme = toml::from_str(content).map_err(VogixError::TomlParse)?;

    let mut colors = HashMap::new();

    // Primary colors
    if let Some(primary) = &theme.colors.primary {
        if let Some(v) = &primary.background {
            colors.insert("background".to_string(), v.clone());
        }
        if let Some(v) = &primary.foreground {
            colors.insert("foreground".to_string(), v.clone());
        }
    }

    // Cursor colors
    if let Some(cursor) = &theme.colors.cursor {
        if let Some(v) = &cursor.cursor {
            colors.insert("cursor_bg".to_string(), v.clone());
        }
        if let Some(v) = &cursor.text {
            colors.insert("cursor_fg".to_string(), v.clone());
        }
    }

    // Selection colors
    if let Some(selection) = &theme.colors.selection {
        if let Some(v) = &selection.background {
            colors.insert("selection_bg".to_string(), v.clone());
        }
        if let Some(v) = &selection.text {
            colors.insert("selection_fg".to_string(), v.clone());
        }
    }

    // Normal colors (color00-color07)
    if let Some(normal) = &theme.colors.normal {
        insert_normal_colors(&mut colors, normal);
    }

    // Bright colors (color08-color15)
    if let Some(bright) = &theme.colors.bright {
        insert_bright_colors(&mut colors, bright);
    }

    Ok(colors)
}

fn insert_normal_colors(colors: &mut HashMap<String, String>, normal: &Ansi16Normal) {
    if let Some(v) = &normal.black {
        colors.insert("color00".to_string(), v.clone());
    }
    if let Some(v) = &normal.red {
        colors.insert("color01".to_string(), v.clone());
    }
    if let Some(v) = &normal.green {
        colors.insert("color02".to_string(), v.clone());
    }
    if let Some(v) = &normal.yellow {
        colors.insert("color03".to_string(), v.clone());
    }
    if let Some(v) = &normal.blue {
        colors.insert("color04".to_string(), v.clone());
    }
    if let Some(v) = &normal.magenta {
        colors.insert("color05".to_string(), v.clone());
    }
    if let Some(v) = &normal.cyan {
        colors.insert("color06".to_string(), v.clone());
    }
    if let Some(v) = &normal.white {
        colors.insert("color07".to_string(), v.clone());
    }
}

fn insert_bright_colors(colors: &mut HashMap<String, String>, bright: &Ansi16Bright) {
    if let Some(v) = &bright.black {
        colors.insert("color08".to_string(), v.clone());
    }
    if let Some(v) = &bright.red {
        colors.insert("color09".to_string(), v.clone());
    }
    if let Some(v) = &bright.green {
        colors.insert("color10".to_string(), v.clone());
    }
    if let Some(v) = &bright.yellow {
        colors.insert("color11".to_string(), v.clone());
    }
    if let Some(v) = &bright.blue {
        colors.insert("color12".to_string(), v.clone());
    }
    if let Some(v) = &bright.magenta {
        colors.insert("color13".to_string(), v.clone());
    }
    if let Some(v) = &bright.cyan {
        colors.insert("color14".to_string(), v.clone());
    }
    if let Some(v) = &bright.white {
        colors.insert("color15".to_string(), v.clone());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_full_theme() {
        let content = r##"[colors.primary]
background = "#282a36"
foreground = "#f8f8f2"

[colors.cursor]
cursor = "#f8f8f2"
text = "#282a36"

[colors.selection]
background = "#44475a"
text = "#ffffff"

[colors.normal]
black = "#21222c"
red = "#ff5555"
green = "#50fa7b"
yellow = "#f1fa8c"
blue = "#bd93f9"
magenta = "#ff79c6"
cyan = "#8be9fd"
white = "#f8f8f2"

[colors.bright]
black = "#6272a4"
red = "#ff6e6e"
green = "#69ff94"
yellow = "#ffffa5"
blue = "#d6acff"
magenta = "#ff92df"
cyan = "#a4ffff"
white = "#ffffff"
"##;

        let colors = load(content, Path::new("test.toml")).unwrap();

        // Primary
        assert_eq!(colors.get("background"), Some(&"#282a36".to_string()));
        assert_eq!(colors.get("foreground"), Some(&"#f8f8f2".to_string()));

        // Cursor
        assert_eq!(colors.get("cursor_bg"), Some(&"#f8f8f2".to_string()));
        assert_eq!(colors.get("cursor_fg"), Some(&"#282a36".to_string()));

        // Selection
        assert_eq!(colors.get("selection_bg"), Some(&"#44475a".to_string()));
        assert_eq!(colors.get("selection_fg"), Some(&"#ffffff".to_string()));

        // Normal colors
        assert_eq!(colors.get("color00"), Some(&"#21222c".to_string()));
        assert_eq!(colors.get("color07"), Some(&"#f8f8f2".to_string()));

        // Bright colors
        assert_eq!(colors.get("color08"), Some(&"#6272a4".to_string()));
        assert_eq!(colors.get("color15"), Some(&"#ffffff".to_string()));
    }

    #[test]
    fn test_load_partial_theme() {
        // Only primary and some normal colors
        let content = r##"[colors.primary]
background = "#282a36"
foreground = "#f8f8f2"

[colors.normal]
black = "#21222c"
red = "#ff5555"
"##;

        let colors = load(content, Path::new("test.toml")).unwrap();

        assert_eq!(colors.get("background"), Some(&"#282a36".to_string()));
        assert_eq!(colors.get("color00"), Some(&"#21222c".to_string()));
        assert_eq!(colors.get("color01"), Some(&"#ff5555".to_string()));

        // Missing colors should not be in the map
        assert!(!colors.contains_key("color02"));
        assert!(!colors.contains_key("cursor_bg"));
    }

    #[test]
    fn test_load_invalid_toml() {
        let result = load("not valid toml {{", Path::new("test.toml"));
        assert!(result.is_err());
    }
}

//! Tests for template module

use super::*;
use std::collections::HashMap;
use std::io::Write;
use tempfile::NamedTempFile;

fn sample_colors() -> HashMap<String, String> {
    let mut colors = HashMap::new();
    colors.insert("base00".to_string(), "#1e1e2e".to_string());
    colors.insert("base01".to_string(), "#181825".to_string());
    colors.insert("base02".to_string(), "#313244".to_string());
    colors.insert("base03".to_string(), "#45475a".to_string());
    colors.insert("base04".to_string(), "#585b70".to_string());
    colors.insert("base05".to_string(), "#cdd6f4".to_string());
    colors.insert("base06".to_string(), "#f5e0dc".to_string());
    colors.insert("base07".to_string(), "#b4befe".to_string());
    colors.insert("base08".to_string(), "#f38ba8".to_string());
    colors.insert("base09".to_string(), "#fab387".to_string());
    colors.insert("base0A".to_string(), "#f9e2af".to_string());
    colors.insert("base0B".to_string(), "#a6e3a1".to_string());
    colors.insert("base0C".to_string(), "#94e2d5".to_string());
    colors.insert("base0D".to_string(), "#89b4fa".to_string());
    colors.insert("base0E".to_string(), "#cba6f7".to_string());
    colors.insert("base0F".to_string(), "#f2cdcd".to_string());
    colors
}

#[test]
fn test_render_template_string_simple() {
    let colors = sample_colors();
    let template = r##"background = "{{ colors.base00 }}""##;

    let result = render_template_string(template, &colors, &HashMap::new()).unwrap();
    assert_eq!(result, "background = \"#1e1e2e\"");
}

#[test]
fn test_render_template_string_multiple_colors() {
    let colors = sample_colors();
    let template = r##"[colors.primary]
background = "{{ colors.base00 }}"
foreground = "{{ colors.base05 }}"

[colors.normal]
red = "{{ colors.base08 }}"
green = "{{ colors.base0B }}""##;

    let result = render_template_string(template, &colors, &HashMap::new()).unwrap();
    assert!(result.contains("background = \"#1e1e2e\""));
    assert!(result.contains("foreground = \"#cdd6f4\""));
    assert!(result.contains("red = \"#f38ba8\""));
    assert!(result.contains("green = \"#a6e3a1\""));
}

#[test]
fn test_render_template_string_preserves_non_template_content() {
    let colors = sample_colors();
    let template = r##"[font]
size = 12

[colors.primary]
background = "{{ colors.base00 }}""##;

    let result = render_template_string(template, &colors, &HashMap::new()).unwrap();
    assert!(result.contains("[font]"));
    assert!(result.contains("size = 12"));
    assert!(result.contains("background = \"#1e1e2e\""));
}

#[test]
fn test_render_template_string_missing_variable() {
    let colors = sample_colors();
    let template = r##"background = "{{ colors.nonexistent }}""##;

    let result = render_template_string(template, &colors, &HashMap::new());
    // Tera errors on missing variables by default (strict mode)
    assert!(result.is_err());
}

#[test]
fn test_render_template_string_invalid_syntax() {
    let colors = sample_colors();
    // Missing closing braces
    let template = r##"background = "{{ colors.base00 }""##;

    let result = render_template_string(template, &colors, &HashMap::new());
    assert!(result.is_err());
}

#[test]
fn test_render_template_from_file() {
    let colors = sample_colors();

    let mut template_file = NamedTempFile::new().unwrap();
    write!(
        template_file,
        "[colors.primary]\nbackground = \"{{{{ colors.base00 }}}}\"\nforeground = \"{{{{ colors.base05 }}}}\""
    )
    .unwrap();

    let result = render_template(template_file.path(), &colors, &HashMap::new()).unwrap();
    assert!(result.contains("background = \"#1e1e2e\""));
    assert!(result.contains("foreground = \"#cdd6f4\""));
}

#[test]
fn test_render_template_file_not_found() {
    let colors = sample_colors();
    let result = render_template("/nonexistent/template.vogix", &colors, &HashMap::new());
    assert!(result.is_err());
}

#[test]
fn test_full_alacritty_template() {
    let colors = sample_colors();
    let template = r##"[colors.primary]
background = "{{ colors.base00 }}"
foreground = "{{ colors.base05 }}"

[colors.cursor]
cursor = "{{ colors.base05 }}"
text = "{{ colors.base00 }}"

[colors.normal]
black = "{{ colors.base00 }}"
red = "{{ colors.base08 }}"
green = "{{ colors.base0B }}"
yellow = "{{ colors.base0A }}"
blue = "{{ colors.base0D }}"
magenta = "{{ colors.base0E }}"
cyan = "{{ colors.base0C }}"
white = "{{ colors.base05 }}"

[colors.bright]
black = "{{ colors.base03 }}"
red = "{{ colors.base08 }}"
green = "{{ colors.base0B }}"
yellow = "{{ colors.base0A }}"
blue = "{{ colors.base0D }}"
magenta = "{{ colors.base0E }}"
cyan = "{{ colors.base0C }}"
white = "{{ colors.base07 }}""##;

    let result = render_template_string(template, &colors, &HashMap::new()).unwrap();

    // Verify key color mappings
    assert!(result.contains("background = \"#1e1e2e\""));
    assert!(result.contains("foreground = \"#cdd6f4\""));
    assert!(result.contains("red = \"#f38ba8\""));
    assert!(result.contains("green = \"#a6e3a1\""));
    assert!(result.contains("blue = \"#89b4fa\""));
}

// Integration tests for actual template files
#[test]
fn test_render_base16_alacritty_template_file() {
    let template_path = std::path::Path::new("templates/base16/alacritty.toml.vogix");
    if !template_path.exists() {
        // Skip if templates not present (e.g., in CI without full checkout)
        return;
    }

    let colors = sample_colors();
    let result = render_template(template_path, &colors, &HashMap::new()).unwrap();

    // Verify the template renders correctly
    assert!(result.contains("background = \"#1e1e2e\""));
    assert!(result.contains("foreground = \"#cdd6f4\""));
    assert!(result.contains("[colors.primary]"));
    assert!(result.contains("[colors.normal]"));
    assert!(result.contains("[colors.bright]"));
}

fn sample_vogix16_colors() -> HashMap<String, String> {
    let mut colors = HashMap::new();
    // Monochromatic
    colors.insert("background".to_string(), "#262626".to_string());
    colors.insert("background_surface".to_string(), "#333333".to_string());
    colors.insert("background_selection".to_string(), "#4d4d4d".to_string());
    colors.insert("foreground_comment".to_string(), "#666666".to_string());
    colors.insert("foreground_border".to_string(), "#808080".to_string());
    colors.insert("foreground_text".to_string(), "#cccccc".to_string());
    colors.insert("foreground_heading".to_string(), "#e6e6e6".to_string());
    colors.insert("foreground_bright".to_string(), "#ffffff".to_string());
    // Functional
    colors.insert("danger".to_string(), "#e06c75".to_string());
    colors.insert("warning".to_string(), "#e5c07b".to_string());
    colors.insert("notice".to_string(), "#d19a66".to_string());
    colors.insert("success".to_string(), "#98c379".to_string());
    colors.insert("active".to_string(), "#56b6c2".to_string());
    colors.insert("link".to_string(), "#61afef".to_string());
    colors.insert("highlight".to_string(), "#c678dd".to_string());
    colors.insert("special".to_string(), "#be5046".to_string());
    colors
}

#[test]
fn test_render_vogix16_alacritty_template_file() {
    let template_path = std::path::Path::new("templates/vogix16/alacritty.toml.vogix");
    if !template_path.exists() {
        return;
    }

    let colors = sample_vogix16_colors();
    let result = render_template(template_path, &colors, &HashMap::new()).unwrap();

    // Verify semantic color mappings
    assert!(result.contains("background = \"#262626\""));
    assert!(result.contains("foreground = \"#cccccc\""));
    assert!(result.contains("red = \"#e06c75\"")); // danger
    assert!(result.contains("green = \"#98c379\"")); // success
}

fn sample_ansi16_colors() -> HashMap<String, String> {
    let mut colors = HashMap::new();
    colors.insert("background".to_string(), "#1d1f21".to_string());
    colors.insert("foreground".to_string(), "#c5c8c6".to_string());
    colors.insert("cursor_bg".to_string(), "#c5c8c6".to_string());
    colors.insert("cursor_fg".to_string(), "#1d1f21".to_string());
    colors.insert("selection_bg".to_string(), "#373b41".to_string());
    colors.insert("selection_fg".to_string(), "#c5c8c6".to_string());
    // Normal colors
    colors.insert("color00".to_string(), "#1d1f21".to_string());
    colors.insert("color01".to_string(), "#cc6666".to_string());
    colors.insert("color02".to_string(), "#b5bd68".to_string());
    colors.insert("color03".to_string(), "#f0c674".to_string());
    colors.insert("color04".to_string(), "#81a2be".to_string());
    colors.insert("color05".to_string(), "#b294bb".to_string());
    colors.insert("color06".to_string(), "#8abeb7".to_string());
    colors.insert("color07".to_string(), "#c5c8c6".to_string());
    // Bright colors
    colors.insert("color08".to_string(), "#969896".to_string());
    colors.insert("color09".to_string(), "#cc6666".to_string());
    colors.insert("color10".to_string(), "#b5bd68".to_string());
    colors.insert("color11".to_string(), "#f0c674".to_string());
    colors.insert("color12".to_string(), "#81a2be".to_string());
    colors.insert("color13".to_string(), "#b294bb".to_string());
    colors.insert("color14".to_string(), "#8abeb7".to_string());
    colors.insert("color15".to_string(), "#ffffff".to_string());
    colors
}

#[test]
fn test_render_ansi16_alacritty_template_file() {
    let template_path = std::path::Path::new("templates/ansi16/alacritty.toml.vogix");
    if !template_path.exists() {
        return;
    }

    let colors = sample_ansi16_colors();
    let result = render_template(template_path, &colors, &HashMap::new()).unwrap();

    // Verify ANSI color mappings
    assert!(result.contains("background = \"#1d1f21\""));
    assert!(result.contains("foreground = \"#c5c8c6\""));
    assert!(result.contains("red = \"#cc6666\"")); // color01
    assert!(result.contains("green = \"#b5bd68\"")); // color02
}

#[test]
fn test_hex_to_rgb_filter() {
    let mut colors = HashMap::new();
    colors.insert("red".to_string(), "#FF5733".to_string());

    let template = "{{ colors.red | hex_to_rgb }}";
    let result = render_template_string(template, &colors, &HashMap::new()).unwrap();

    assert_eq!(result, "0xFF,0x57,0x33");
}

#[test]
fn test_strip_hash_filter() {
    let mut colors = HashMap::new();
    colors.insert("blue".to_string(), "#1e90ff".to_string());

    let template = "{{ colors.blue | strip_hash }}";
    let result = render_template_string(template, &colors, &HashMap::new()).unwrap();

    assert_eq!(result, "1e90ff");
}

#[test]
fn test_hex_to_rgb_filter_lowercase() {
    let mut colors = HashMap::new();
    colors.insert("color".to_string(), "#abcdef".to_string());

    let template = "{{ colors.color | hex_to_rgb }}";
    let result = render_template_string(template, &colors, &HashMap::new()).unwrap();

    assert_eq!(result, "0xab,0xcd,0xef");
}

/// The theme.json contract template must render BYTE-IDENTICAL to the Nix
/// generator's `builtins.toJSON` output — the desktop shell reads whichever
/// render layer produced the file (Nix-built theme package or the on-demand
/// cache), so the two may never drift. The golden line below is the Nix
/// generator's output for the same fixture (nix/modules/contract-tests.nix
/// pins the Nix side to the identical string).
#[test]
fn theme_json_template_matches_nix_generator_bytes() {
    let template = include_str!("../../templates/vogix16/theme.json.vogix");

    // base00..base0F = #101010..#1f1f1f, keyed by the SNAKE semantic names
    // the runtime colors map carries for vogix16.
    let semantic_by_slot = [
        "background",
        "background_surface",
        "background_selection",
        "foreground_comment",
        "foreground_border",
        "foreground_text",
        "foreground_heading",
        "foreground_bright",
        "success",
        "warning",
        "notice",
        "danger",
        "active",
        "link",
        "highlight",
        "special",
    ];
    let colors: HashMap<String, String> = semantic_by_slot
        .iter()
        .enumerate()
        .map(|(i, key)| {
            let b = format!("{:02x}", 0x10 + i);
            (key.to_string(), format!("#{b}{b}{b}"))
        })
        .collect();
    let meta = HashMap::from([
        ("theme".to_string(), "goldtest".to_string()),
        ("variant".to_string(), "night".to_string()),
        ("scheme".to_string(), "vogix16".to_string()),
        ("polarity".to_string(), "dark".to_string()),
    ]);

    let rendered = render_template_string(template, &colors, &meta).unwrap();
    let golden = concat!(
        r##"{"backgrounds":[],"palette":{"base00":"#101010","base01":"#111111","base02":"#121212","base03":"#131313","base04":"#141414","base05":"#151515","base06":"#161616","base07":"#171717","base08":"#181818","base09":"#191919","base0A":"#1a1a1a","base0B":"#1b1b1b","base0C":"#1c1c1c","base0D":"#1d1d1d","base0E":"#1e1e1e","base0F":"#1f1f1f"},"polarity":"dark","schema":1,"scheme":"vogix16","semantic":{"active":"#1c1c1c","background":"#101010","background_selection":"#121212","background_surface":"#111111","danger":"#1b1b1b","foreground_border":"#141414","foreground_bright":"#171717","foreground_comment":"#131313","foreground_heading":"#161616","foreground_text":"#151515","highlight":"#1e1e1e","link":"#1d1d1d","notice":"#1a1a1a","special":"#1f1f1f","success":"#181818","warning":"#191919"},"theme":"goldtest","variant":"night"}"##,
        "\n"
    );
    assert_eq!(rendered, golden);
}

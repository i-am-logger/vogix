/// Theme validation against praxis axioms — for evaluation.
///
/// Loads base16/base24 YAML themes from the tinted-schemes dataset
/// and validates each against the ontology axioms.
use praxis::category::Entity;
use praxis::ontology::Axiom;
use praxis_domains::science::colors::srgb;
use praxis_domains::science::colors::Rgb;
use praxis_domains::technology::theming::base16::ColorSlot;
use praxis_domains::technology::theming::ontology::{
    LuminanceMonotonicity, Palette, WcagForegroundContrast,
};
use std::path::Path;
/// Result of validating a single theme variant.
#[derive(Debug)]
pub struct ThemeResult {
    pub theme: String,
    pub variant: String,
    pub slots_found: usize,
    pub luminance_monotone: bool,
    pub wcag_aa: bool,
    pub contrast_ratio: Option<f64>,
    pub polarity: String,
}
/// Parse a base16 YAML theme file into a Palette.
pub fn parse_yaml_theme(content: &str) -> Option<Palette> {
    let mut palette = Palette::new();

    let in_palette = content.contains("palette:");
    let lines: Vec<&str> = content.lines().collect();

    let mut reading_palette = !in_palette; // if no palette: key, assume flat format

    for line in &lines {
        let trimmed = line.trim();

        if trimmed.starts_with("palette:") {
            reading_palette = true;
            continue;
        }

        if reading_palette {
            // Stop at next top-level key
            if !trimmed.is_empty()
                && !trimmed.starts_with("base0")
                && !trimmed.starts_with("base1")
                && !trimmed.starts_with('#')
                && !trimmed.starts_with("base")
                && trimmed.contains(':')
                && !trimmed.starts_with(' ')
                && in_palette
            {
                break;
            }

            // Parse baseXX: "#rrggbb"
            for slot in ColorSlot::variants() {
                let key = slot.key();
                if trimmed.starts_with(&format!("{key}:")) || trimmed.starts_with(&format!("{key} :"))
                {
                    // Extract hex value — handle "base00: #1e1e2e" and 'base00: "#1e1e2e"'
                    if let Some(hex_part) = trimmed.split(':').nth(1) {
                        let hex = hex_part
                            .trim()
                            .trim_matches('"')
                            .trim_matches('\'')
                            .split_whitespace()
                            .next()
                            .unwrap_or("");
                        let hex = if hex.starts_with('#') { hex } else { &format!("#{hex}") };
                        if let Some(rgb) = Rgb::from_hex(hex) {
                            palette.insert(slot, rgb);
                        }
                    }
                }
            }
        }
    }

    if palette.is_empty() {
        None
    } else {
        Some(palette)
    }
}
/// Validate a palette against all axioms.
pub fn validate_palette(palette: &Palette) -> (bool, bool, Option<f64>) {
    let mono = LuminanceMonotonicity {
        palette: palette.clone(),
    };
    let contrast = WcagForegroundContrast {
        palette: palette.clone(),
    };

    let cr = match (palette.get(&ColorSlot::Base00), palette.get(&ColorSlot::Base05)) {
        (Some(bg), Some(fg)) => Some(srgb::contrast_ratio(fg, bg)),
        _ => None,
    };

    (mono.holds(), contrast.holds(), cr)
}
/// Scan a directory of base16 themes and validate each.
pub fn scan_themes(base_dir: &Path) -> Vec<ThemeResult> {
    let mut results = Vec::new();

    let Ok(entries) = std::fs::read_dir(base_dir) else {
        return results;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let theme_name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        // Each theme dir can have multiple variant YAML files
        let Ok(variants) = std::fs::read_dir(&path) else {
            continue;
        };

        for variant_entry in variants.flatten() {
            let vpath = variant_entry.path();
            if vpath.extension().is_some_and(|e| e == "yaml" || e == "yml") {
                let variant_name = vpath
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();

                let Ok(content) = std::fs::read_to_string(&vpath) else {
                    continue;
                };

                let Some(palette) = parse_yaml_theme(&content) else {
                    continue;
                };

                let (mono, wcag, cr) = validate_palette(&palette);

                let polarity = match palette.get(&ColorSlot::Base00) {
                    Some(bg) => {
                        if srgb::is_dark(bg) {
                            "dark"
                        } else {
                            "light"
                        }
                    }
                    None => "unknown",
                };

                results.push(ThemeResult {
                    theme: theme_name.clone(),
                    variant: variant_name,
                    slots_found: palette.len(),
                    luminance_monotone: mono,
                    wcag_aa: wcag,
                    contrast_ratio: cr,
                    polarity: polarity.to_string(),
                });
            }
        }
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_catppuccin_mocha() {
        let yaml = r##"
system: "base16"
name: "Catppuccin Mocha"
author: "https://github.com/catppuccin/catppuccin"
variant: "dark"
palette:
  base00: "#1e1e2e"
  base01: "#313244"
  base02: "#45475a"
  base03: "#6c7086"
  base04: "#a6adc8"
  base05: "#cdd6f4"
  base06: "#f5e0dc"
  base07: "#b4befe"
  base08: "#f38ba8"
  base09: "#fab387"
  base0A: "#f9e2af"
  base0B: "#a6e3a1"
  base0C: "#94e2d5"
  base0D: "#89b4fa"
  base0E: "#cba6f7"
  base0F: "#f2cdcd"
"##;
        let palette = parse_yaml_theme(yaml).unwrap();
        assert_eq!(palette.len(), 16);
    }

    #[test]
    fn test_catppuccin_monotonicity() {
        // Catppuccin Mocha base06 (rosewater) and base07 (lavender) have
        // lower luminance than base05 (text), so strict monotonicity fails.
        // This is a real finding — many popular themes violate this axiom.
        let yaml = r##"
palette:
  base00: "#1e1e2e"
  base01: "#313244"
  base02: "#45475a"
  base03: "#6c7086"
  base04: "#a6adc8"
  base05: "#cdd6f4"
  base06: "#f5e0dc"
  base07: "#b4befe"
  base08: "#f38ba8"
  base09: "#fab387"
  base0A: "#f9e2af"
  base0B: "#a6e3a1"
  base0C: "#94e2d5"
  base0D: "#89b4fa"
  base0E: "#cba6f7"
  base0F: "#f2cdcd"
"##;
        let palette = parse_yaml_theme(yaml).unwrap();
        let (mono, _wcag, _cr) = validate_palette(&palette);
        // Catppuccin Mocha fails monotonicity: base06 (rosewater) < base05 (text)
        assert!(!mono);
    }

    #[test]
    fn test_bad_contrast_detected() {
        let yaml = r##"
palette:
  base00: "#1e1e2e"
  base01: "#202030"
  base02: "#252535"
  base03: "#303040"
  base04: "#353545"
  base05: "#2a2a3a"
  base06: "#2f2f3f"
  base07: "#343444"
  base08: "#ff0000"
  base09: "#ff8800"
  base0A: "#ffff00"
  base0B: "#00ff00"
  base0C: "#00ffff"
  base0D: "#0000ff"
  base0E: "#ff00ff"
  base0F: "#884400"
"##;
        let palette = parse_yaml_theme(yaml).unwrap();
        let (_mono, wcag, cr) = validate_palette(&palette);
        assert!(!wcag, "should fail WCAG AA with similar fg/bg");
        assert!(cr.unwrap() < 4.5);
    }

    #[test]
    fn test_scan_real_themes() {
        let base16_dir = std::path::Path::new(env!("HOME"))
            .join("Code/github/logger/tinted-schemes/base16");

        if !base16_dir.exists() {
            return; // skip if dataset not available
        }

        let results = scan_themes(&base16_dir);
        assert!(!results.is_empty(), "should find themes");

        let total = results.len();
        let mono_pass = results.iter().filter(|r| r.luminance_monotone).count();
        let wcag_pass = results.iter().filter(|r| r.wcag_aa).count();
        let dark_count = results.iter().filter(|r| r.polarity == "dark").count();
        let light_count = results.iter().filter(|r| r.polarity == "light").count();

        println!("\n═══════════════════════════════════");
        println!("  Theme Validation Results");
        println!("═══════════════════════════════════");
        println!("  Total themes:           {}", total);
        println!("  Luminance monotone:     {} ({:.0}%)", mono_pass, mono_pass as f64 / total as f64 * 100.0);
        println!("  WCAG AA compliant:      {} ({:.0}%)", wcag_pass, wcag_pass as f64 / total as f64 * 100.0);
        println!("  Dark themes:            {}", dark_count);
        println!("  Light themes:           {}", light_count);
        println!("═══════════════════════════════════");

        // Print failures
        let mono_failures: Vec<_> = results.iter().filter(|r| !r.luminance_monotone).collect();
        if !mono_failures.is_empty() {
            println!("\n  Luminance monotonicity failures:");
            for r in &mono_failures[..mono_failures.len().min(10)] {
                println!("    - {}/{}", r.theme, r.variant);
            }
            if mono_failures.len() > 10 {
                println!("    ... and {} more", mono_failures.len() - 10);
            }
        }

        let wcag_failures: Vec<_> = results.iter().filter(|r| !r.wcag_aa).collect();
        if !wcag_failures.is_empty() {
            println!("\n  WCAG AA failures (fg/bg contrast < 4.5:1):");
            for r in &wcag_failures[..wcag_failures.len().min(10)] {
                println!(
                    "    - {}/{} (CR: {:.2}:1)",
                    r.theme,
                    r.variant,
                    r.contrast_ratio.unwrap_or(0.0)
                );
            }
            if wcag_failures.len() > 10 {
                println!("    ... and {} more", wcag_failures.len() - 10);
            }
        }
        println!();
    }
}

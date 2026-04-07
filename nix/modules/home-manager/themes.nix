# Theme imports and utilities for Vogix Home Manager module
#
# Provides:
# - Theme imports from all scheme sources (vogix16, base16, base24, ansi16)
# - Helper functions for theme/variant resolution
{ lib
, tintedSchemes
, iterm2Schemes
, vogix16Themes
,
}:

let
  # Import base16 and base24 themes from tinted-theming/schemes
  base16Import = import ../base16-import.nix {
    inherit lib tintedSchemes;
    scheme = "base16";
  };
  base24Import = import ../base16-import.nix {
    inherit lib tintedSchemes;
    scheme = "base24";
  };

  # Import ansi16 themes from iTerm2-Color-Schemes
  ansi16Import = import ../ansi16-import.nix {
    inherit lib iterm2Schemes;
  };

  # Import vogix16 themes from vogix16-themes
  vogix16Import = import ../vogix16-import.nix {
    inherit lib vogix16Themes;
  };

  vogix16Themes' = vogix16Import.themes;
  base16Themes = base16Import.themes;
  base24Themes = base24Import.themes;
  ansi16Themes = ansi16Import.themes;

  # Combined themes: vogix16 > base16 > base24 > ansi16 (precedence order)
  # User themes override all
  mergeThemes =
    userThemes: ansi16Themes // base24Themes // base16Themes // vogix16Themes' // userThemes;

  # Helper to get the actual variant name for a theme given a polarity
  # Uses theme.defaults to map polarity -> variant name
  # Falls back to polarity if defaults doesn't exist or doesn't have the polarity
  getVariantName = theme: polarity: theme.defaults.${polarity} or polarity;

  # Helper to get theme for each app (with per-app override support)
  getAppTheme = cfg: app: if cfg.${app}.theme != null then cfg.${app}.theme else cfg.appearance.theme;

  # Helper to get variant for each app (with per-app override support)
  # Returns the polarity (dark/light), not the variant name
  getAppVariant = cfg: app: if cfg.${app}.variant != null then cfg.${app}.variant else cfg.appearance.variant;

in
{
  inherit
    mergeThemes
    getVariantName
    getAppTheme
    getAppVariant
    ;

  # Individual scheme themes (for debugging/testing)
  vogix16Themes = vogix16Themes';
  inherit base16Themes base24Themes ansi16Themes;
}

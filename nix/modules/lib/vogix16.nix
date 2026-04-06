# vogix16 scheme utilities
#
# Contains vogix16-specific logic:
# - Semantic color mapping (base00-0F -> functional names)
# - Future vogix16 helpers
_:

{
  # Create semantic color mapping from baseXX colors
  # This provides a clean API for application modules using vogix16 semantic names
  #
  # Monochromatic scale (base00-07): background -> foreground progression
  # Functional colors (base08-0F): status and accent colors
  semanticColors = baseColors: {
    # Monochromatic base (base00-07)
    background = baseColors.base00;
    background-surface = baseColors.base01;
    background-selection = baseColors.base02;
    foreground-comment = baseColors.base03;
    foreground-border = baseColors.base04;
    foreground-text = baseColors.base05;
    foreground-heading = baseColors.base06;
    foreground-bright = baseColors.base07;

    # Functional colors (base08-0F)
    success = baseColors.base08;
    warning = baseColors.base09;
    notice = baseColors.base0A;
    danger = baseColors.base0B;
    active = baseColors.base0C;
    link = baseColors.base0D;
    highlight = baseColors.base0E;
    special = baseColors.base0F;
  };
}

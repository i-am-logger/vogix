# vogix16 theme importer for directory-based vogix16-themes repo
#
# Imports vogix16 themes from i-am-logger/vogix16-themes
# which uses a directory-based structure:
#   {theme-name}/{variant}.toml (day.toml, night.toml)
#
# Each directory becomes a multi-variant theme with all variants merged.
#
{ lib
, vogix16Themes
,
}:

let
  # Read theme directories (only dirs containing .toml variant files are themes)
  allDirs = builtins.attrNames (
    lib.filterAttrs (_: type: type == "directory") (builtins.readDir vogix16Themes)
  );
  hasTomlFiles = dir:
    builtins.any (f: lib.hasSuffix ".toml" f)
      (builtins.attrNames (builtins.readDir "${vogix16Themes}/${dir}"));
  themeDirs = builtins.filter hasTomlFiles allDirs;

  # Parse a single TOML variant file
  parseVariantFile =
    themeDir: filename:
    let
      content = builtins.readFile "${vogix16Themes}/${themeDir}/${filename}";
      parsed = builtins.fromTOML content;
      variantName = lib.removeSuffix ".toml" filename;
      polarity = parsed.polarity or "dark";
      colors = parsed.colors or { };
    in
    {
      inherit variantName polarity colors;
    };

  # Import a single theme directory with all its variants
  importThemeDir =
    themeDir:
    let
      themePath = "${vogix16Themes}/${themeDir}";
      tomlFiles = builtins.filter (f: lib.hasSuffix ".toml" f) (
        builtins.attrNames (builtins.readDir themePath)
      );

      # Parse all variant files
      parsedVariants = map (parseVariantFile themeDir) tomlFiles;

      # Build variants attribute set
      variants = lib.listToAttrs (
        map
          (
            v:
            lib.nameValuePair v.variantName {
              inherit (v) polarity colors;
            }
          )
          parsedVariants
      );

      # Build defaults mapping (polarity -> variant name)
      # For vogix16: night = dark, day = light
      nightVariant = lib.findFirst (v: v.polarity == "dark") null parsedVariants;
      dayVariant = lib.findFirst (v: v.polarity == "light") null parsedVariants;

      defaults =
        (lib.optionalAttrs (nightVariant != null) { dark = nightVariant.variantName; })
        // (lib.optionalAttrs (dayVariant != null) { light = dayVariant.variantName; });
    in
    {
      slug = themeDir;
      name = themeDir;
      inherit variants defaults;
    };

  # Import all theme directories
  allThemes = map importThemeDir themeDirs;

  # Convert to final multi-variant format
  toMultiVariantTheme = theme: {
    inherit (theme) name variants defaults;
    scheme = "vogix16";
    author = "vogix16";
  };

  # Create the final attribute set: { slug = theme; ... }
  importedThemes = lib.listToAttrs (
    map (theme: lib.nameValuePair theme.slug (toMultiVariantTheme theme)) allThemes
  );

in
{
  # Expose the imported themes
  themes = importedThemes;

  # Expose individual themes for debugging
  inherit allThemes;

  # Helper to get theme count
  count = builtins.length allThemes;
}

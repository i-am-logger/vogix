# Theme package generators for Vogix Home Manager module
#
# Generates /nix/store packages for each theme-variant combination
{ lib, pkgs }:

let
  inherit (lib)
    concatMapStringsSep
    mapAttrs
    optionalString
    ;

  # Import vogix16-specific utilities (semantic color mapping)
  vogix16Lib = import ../lib/vogix16.nix { inherit lib; };
  inherit (vogix16Lib) semanticColors base16FromAnsi16;

  backgroundsLib = import ../backgrounds { inherit lib pkgs; };
  inherit (backgroundsLib) mkGeneratedBackground;

  # Import shared application discovery
  appDiscovery = import ../lib/applications.nix { inherit lib; };
  inherit (appDiscovery) availableApps appGenerators;

  # Helper: Check if an app should be themed
  # Theme apps where vogix.<app>.enable = true (defaults to true)
  # We no longer require programs.<app>.enable because that conflicts with vogix's
  # config file management.
  isAppEnabled =
    _config: cfg: appName:
      cfg.${appName}.enable or true;

  # Helper to get format generator for an app
  getFormatGen =
    app:
    let
      appModule = appGenerators.${app} or null;
      format = if appModule != null then appModule.format or "toml" else "toml";
    in
    if format == "toml" then
      pkgs.formats.toml { }
    else if format == "yaml" then
      pkgs.formats.yaml { }
    else if format == "json" then
      pkgs.formats.json { }
    else if format == "custom-bat" then
      {
        # Bat config format: key=value (one per line)
        generate =
          name: attrs:
          pkgs.writeText name (
            lib.concatStringsSep "\n" (lib.mapAttrsToList (k: v: "${k}=${toString v}") attrs)
          );
      }
    else if format == "custom-btop" then
      {
        # Btop config format: key=value with quotes for strings
        generate =
          name: attrs:
          pkgs.writeText name (
            lib.concatStringsSep "\n" (
              lib.mapAttrsToList
                (
                  k: v:
                  if builtins.isString v then
                    "${k}=\"${v}\""
                  else if builtins.isBool v then
                    "${k}=${if v then "true" else "false"}"
                  else
                    "${k}=${toString v}"
                )
                attrs
            )
          );
      }
    else
      pkgs.formats.toml { }; # default to TOML

  # Generate a single app's config files within a theme package
  generateAppConfig =
    { config
    , colors
    , scheme
    , meta
    ,
    }:
    app:
    let
      appModule = appGenerators.${app} or null;
      # Support both new API (schemes) and old API (generate)
      appSchemes = if appModule != null then appModule.schemes or null else null;
      legacyGenerator = if appModule != null then appModule.generate or null else null;
      generator =
        if appSchemes != null && appSchemes ? ${scheme} then
          appSchemes.${scheme}
        else if legacyGenerator != null && scheme == "vogix16" then
          legacyGenerator
        else
          null;
      settingsPath = if appModule != null then appModule.settingsPath or null else null;
      configFileName = if appModule != null then appModule.configFile or "config" else "config";
      configDir = dirOf configFileName;
    in
    optionalString (appModule != null && generator != null) (
      let
        # A generator written as `{ colors, meta }:` additionally receives the
        # theme identity (theme/variant/scheme/polarity) — the callPackage
        # idiom, detected with functionArgs. Existing `colors:` generators are
        # untouched.
        generatedOutput =
          if builtins.functionArgs generator ? meta
          then generator { inherit colors meta; }
          else generator colors;
        isHybrid = builtins.isAttrs generatedOutput && generatedOutput ? themeFile;
        isSettingsBased = settingsPath != null && !isHybrid;
        themeFileName = if appModule != null then appModule.themeFile or null else null;
      in
      # Three cases: hybrid (theme+settings), settings-only, or theme-file-only
      if isHybrid then
      # HYBRID app (bat, btop): Generate BOTH theme file AND config file with merged settings
        let
          themeFileDir = dirOf themeFileName;
          globalSettings = if appModule != null then appModule.settings or { } else { };
          schemeSettings = generatedOutput.settings or { };
          pathParts = if settingsPath != null then lib.splitString "." settingsPath else [ ];
          userSettings = lib.attrByPath pathParts { } config;
          mergedSettings = lib.recursiveUpdate (lib.recursiveUpdate userSettings globalSettings) schemeSettings;
          formatGen = getFormatGen app;
          configFile = formatGen.generate "vogix-${app}-config" mergedSettings;
        in
        ''
          # Generate theme file
          mkdir -p "$out/${app}/${themeFileDir}"
          cat > "$out/${app}/${themeFileName}" <<'EOF'
          ${generatedOutput.themeFile}
          EOF

          # Generate config file with merged settings
          mkdir -p "$out/${app}/${configDir}"
          cp "${configFile}" "$out/${app}/${configFileName}"
        ''
      else if isSettingsBased then
      # SETTINGS-ONLY app (alacritty): merge user settings with theme colors
        let
          pathParts = if settingsPath != null then lib.splitString "." settingsPath else [ ];
          userSettings = lib.attrByPath pathParts { } config;
          mergedSettings = lib.recursiveUpdate userSettings generatedOutput;
          formatGen = getFormatGen app;
          configFile = formatGen.generate "vogix-${app}-config" mergedSettings;
        in
        ''
          mkdir -p "$out/${app}/${configDir}"
          cp "${configFile}" "$out/${app}/${configFileName}"
        ''
      else
      # THEME-FILE-ONLY app (console, ripgrep): direct string output
        ''
          mkdir -p "$out/${app}/${configDir}"
          cat > "$out/${app}/${configFileName}" <<'EOF'
          ${generatedOutput}
          EOF
        ''
    );

  # Generate theme-variant packages using merged settings
  # Returns: { themeName = { variantName = derivation; ... }; ... }
  mkThemeVariantPackages =
    { config
    , cfg
    , allThemes
    , extraBackgrounds ? { }
    ,
    }:
    let
      themedApps = builtins.filter (isAppEnabled config cfg) availableApps;
    in
    mapAttrs
      (
        themeName: theme:
        mapAttrs
          (
            variantName: variantData:
            let
              rawColors = variantData.colors;
              scheme = theme.scheme or "vogix16";
              colors = if scheme == "vogix16" then semanticColors rawColors else rawColors;
              themeVariantName = "${themeName}-${variantName}";

              # ── Backgrounds (desktop users only) ──
              # The generated "veil" is always present — rendered from THIS
              # variant's 16 base slots (ansi16 goes through the praxis slot
              # fill), so light/dark rides the variant axis for free. Curated
              # entries (the vogix-backgrounds data repo, host additions)
              # merge in through `extraBackgrounds.<theme>.<variant>`.
              #
              # The list lives in backgrounds.json BESIDE theme.json, not
              # inside it: entries carry store paths only Nix can know, and
              # theme.json must stay byte-identical between the Nix and
              # on-demand cache render layers.
              wantBackgrounds = builtins.elem "vogix-desktop" themedApps;
              baseSlotColors =
                if scheme == "ansi16" then base16FromAnsi16 rawColors else rawColors;
              generated = mkGeneratedBackground {
                themeVariant = themeVariantName;
                colors = baseSlotColors;
              };
              backgroundEntries =
                [{
                  kind = "generated";
                  name = "veil";
                  path = "${generated}";
                }]
                ++ ((extraBackgrounds.${themeName} or { }).${variantName} or [ ]);
              backgroundsJson = builtins.toJSON {
                schema = 1;
                backgrounds = backgroundEntries;
              };
              backgroundsSetup = optionalString wantBackgrounds ''
                mkdir -p "$out/vogix-desktop/backgrounds"
                ${concatMapStringsSep "\n"
                  (e: ''ln -s ${lib.escapeShellArg e.path} "$out/vogix-desktop/backgrounds/${e.name}"'')
                  backgroundEntries}
                cp ${pkgs.writeText "vogix-backgrounds-${themeVariantName}.json" backgroundsJson} \
                  "$out/vogix-desktop/backgrounds.json"
              '';
              # The theme identity, for `{ colors, meta }:` generators.
              # Polarity comes from the imported theme data — the same value
              # the config.toml manifest carries, which is what the runtime
              # reads, so the two render layers cannot disagree on it.
              meta = {
                theme = themeName;
                variant = variantName;
                inherit scheme;
                polarity = variantData.polarity or "dark";
              };
            in
            pkgs.runCommand "vogix-theme-${themeVariantName}" { } ''
              mkdir -p $out
              ${concatMapStringsSep "\n" (generateAppConfig {
                inherit
                  config
                  colors
                  scheme
                  meta
                  ;
              }) themedApps}
              ${backgroundsSetup}
            ''
          )
          theme.variants
      )
      allThemes;

in
{
  inherit
    mkThemeVariantPackages
    appGenerators
    availableApps
    isAppEnabled
    ;
}

# Home Manager module for Vogix
#
# Provides:
# - Theme packages in ~/.local/share/vogix/themes/
# - App config symlinks pointing to ~/.local/state/vogix/current-theme/
# - Config file at ~/.local/state/vogix/config.toml
#
# Accepts scheme sources for theme import:
# - tintedSchemes: base16/base24 from tinted-theming/schemes
# - iterm2Schemes: ansi16 from iTerm2-Color-Schemes
# - vogix16Themes: vogix16 from vogix16-themes
{ tintedSchemes
, iterm2Schemes
, vogix16Themes
,
}:

{ config
, lib
, pkgs
, ...
}:

let
  inherit (lib)
    mkIf
    mkMerge
    nameValuePair
    listToAttrs
    flatten
    mapAttrsToList
    concatMapStringsSep
    optionalString
    ;

  cfg = config.programs.vogix;
  acfg = cfg.appearance;

  # Import theme utilities
  themeUtils = import ./themes.nix {
    inherit
      lib
      tintedSchemes
      iterm2Schemes
      vogix16Themes
      ;
  };

  # Import vogix16-specific utilities
  vogix16Lib = import ../lib/vogix16.nix { inherit lib; };

  # Import color utilities
  colorLib = import ../lib/colors.nix { inherit lib; };
  inherit (colorLib) hexToLuminance;

  # Import options module
  optionsModule = import ./options.nix { inherit lib pkgs; };

  # Import generators
  generators = import ./generators.nix { inherit lib pkgs; };
  inherit (generators) appGenerators availableApps isAppEnabled;

  # Import behavior module
  behaviorModule = import ../behavior { inherit lib pkgs; };
  behaviorCfg = cfg.behavior;
  behaviorDefaults = behaviorModule.defaults;


  # Merge all themes with user themes
  allThemes = themeUtils.mergeThemes acfg.themes;

  # Get selected theme colors for the color API
  selectedTheme = allThemes.${acfg.theme};
  selectedVariantName = selectedTheme.defaults.${acfg.variant} or acfg.variant;
  selectedColors = selectedTheme.variants.${selectedVariantName}.colors;

  # Themes staged into the store ahead of time. Every entry costs a derivation
  # per variant plus one per themed app inside it, so this is the difference
  # between a handful of builds and thousands. The selected theme is always
  # included -- the activation symlink points at it, so leaving it out would
  # produce a configuration that cannot start.
  prebuiltThemes =
    if acfg.prebuiltThemes == null then
      allThemes
    else
      lib.getAttrs
        (lib.unique ([ acfg.theme ] ++ (builtins.filter (n: allThemes ? ${n}) acfg.prebuiltThemes)))
        allThemes;

  # Generate theme packages
  themeVariantPackages = generators.mkThemeVariantPackages {
    inherit config cfg;
    allThemes = prebuiltThemes;
    inherit (cfg.appearance) extraBackgrounds;
  };

  # Apps that will be themed
  themedApps = builtins.filter (isAppEnabled config cfg) availableApps;

  # Default theme-variant for initial current-theme symlink
  defaultThemeVariant = "${acfg.theme}-${themeUtils.getVariantName selectedTheme acfg.variant}";

  # Scheme source paths (pointing to /nix/store)
  schemeSources = {
    vogix16 = vogix16Themes;
    base16 = "${tintedSchemes}/base16";
    base24 = "${tintedSchemes}/base24";
    ansi16 = "${iterm2Schemes}/ansi16";
  };

  # Package templates into /nix/store
  templatesDir = builtins.path {
    path = ../../../templates;
    name = "vogix-templates-src";
  };
  templatesPackage = pkgs.runCommand "vogix-templates" { } ''
    mkdir -p $out
    cp -r ${templatesDir}/* $out/
  '';

  # Compute templates hash for cache invalidation.
  #
  # Derived from templatesDir, which is a `builtins.path`: its store path
  # already carries a NAR hash of the template sources, so this changes exactly
  # when a template changes and is stable otherwise. The runtime treats the
  # value as an opaque cache directory name (~/.cache/vogix/themes/<hash>/...)
  # and never recomputes it, so the digest only has to be stable, not to match
  # any particular formula.
  #
  # Reading it out of a derivation instead would be import-from-derivation,
  # which forces a build during evaluation. That breaks every consumer
  # evaluating on a store that does not already have the result -- notably
  # `nix flake check --no-build` in CI, where it fails with "path
  # '...-vogix-templates-hash.drv' is not valid".
  templatesHash = builtins.hashString "sha256" "${templatesDir}";

  # Generate themes section for config.toml
  themesSection = concatMapStringsSep "\n\n"
    (
      themeName:
      let
        theme = allThemes.${themeName};
        scheme = theme.scheme or "vogix16";
        inherit (theme) variants;
        variantNames = builtins.attrNames variants;

        variantLuminance =
          variantName:
          let
            inherit (variants.${variantName}) colors;
            bg = colors.base00 or colors.background or "#000000";
          in
          hexToLuminance bg;

        sortedVariants = builtins.sort (a: b: variantLuminance a > variantLuminance b) variantNames;
        variantOrder = variantName: lib.lists.findFirstIndex (v: v == variantName) 0 sortedVariants;

        variantDetails = lib.concatMapStringsSep "\n"
          (
            variantName:
            let
              variant = variants.${variantName};
              polarity = variant.polarity or "dark";
              order = variantOrder variantName;
            in
            "${variantName} = { polarity = \"${polarity}\", order = ${toString order} }"
          )
          variantNames;
      in
      ''
        [themes."${themeName}"]
        scheme = "${scheme}"
        variants = [${lib.concatMapStringsSep ", " (v: "\"${v}\"") variantNames}]
        ${variantDetails}''
    )
    (builtins.attrNames allThemes);

  # Generate apps section for config.toml with FULL paths
  appsSection = concatMapStringsSep "\n\n"
    (
      app:
      let
        appModule = appGenerators.${app} or null;
        configFileName = if appModule != null then appModule.configFile or "config" else "config";
        themeFileName = if appModule != null then appModule.themeFile or null else null;
        reloadMethod = if appModule != null then appModule.reloadMethod or null else null;
        dataDir = if appModule != null then appModule.dataDir or null else null;
        # Full path to the config file symlink
        configPath =
          if dataDir != null
          then "${config.xdg.dataHome}/${dataDir}/${configFileName}"
          else "${config.xdg.configHome}/${app}/${configFileName}";
        themeFilePath =
          if themeFileName != null then "${config.xdg.configHome}/${app}/${themeFileName}" else null;
      in
      optionalString (appModule != null && reloadMethod != null) ''
        [apps."${app}"]
        config_path = "${configPath}"
        reload_method = "${reloadMethod.method}"
        ${optionalString (themeFilePath != null) "theme_file_path = \"${themeFilePath}\""}
        ${optionalString (reloadMethod ? signal) "reload_signal = \"${reloadMethod.signal}\""}
        ${optionalString (reloadMethod ? process_name) "process_name = \"${reloadMethod.process_name}\""}
        ${optionalString (reloadMethod ? command) "reload_command = \"\"\"${reloadMethod.command}\"\"\""}''
    )
    themedApps;

  # Hardware theme apply commands section
  themeApplySection = concatMapStringsSep "\n\n"
    (name: ''
      [hardware."${name}"]
      command = """${cfg.themeApply.${name}}"""'')
    (builtins.attrNames cfg.themeApply);

  # Shader config section
  shaderSection = optionalString cfg.appearance.shader.enable ''

    # Monochromatic screen shader (auto-generated from theme palette)
    [shader]
    enabled = true
    intensity = ${toString cfg.appearance.shader.intensity}
    brightness = ${toString cfg.appearance.shader.brightness}
    saturation = ${toString cfg.appearance.shader.saturation}
  '';

  # Generate full config.toml content
  configToml = ''
    # Vogix Theme Configuration
    # Auto-generated by home-manager module

    [default]
    theme = "${acfg.theme}"
    variant = "${themeUtils.getVariantName selectedTheme acfg.variant}"

    # Templates for runtime rendering
    [templates]
    path = "${templatesPackage}"
    hash = "${templatesHash}"

    # Theme source directories
    [theme_sources]
    vogix16 = "${schemeSources.vogix16}"
    base16 = "${schemeSources.base16}"
    base24 = "${schemeSources.base24}"
    ansi16 = "${schemeSources.ansi16}"
    ${shaderSection}
    ${themesSection}

    # Application reload methods
    ${appsSection}

    # Hardware theme apply
    ${themeApplySection}
  '';

in
{
  imports = [
    ../hyprland.nix
  ];

  inherit (optionsModule) options;

  config = mkIf cfg.enable (mkMerge [
    # The theme.json contract is generated only for desktop users: with the
    # shell off, terminal-only profiles get no vogix-desktop/ in their theme
    # packages, no [apps."vogix-desktop"] reload entry and no
    # ~/.config/vogix-desktop symlink. Unconditional element (mkDefault, so a
    # user can still force the contract on without the shell).
    {
      programs.vogix."vogix-desktop".enable = lib.mkDefault cfg.desktop.enable;
    }

    # The desktop shell: desktop.json (the per-user layout/token contract),
    # the quickshell config registration, and the vogix-owned unit. The UNIT
    # NAME, the contract file paths and the `vogix desktop` verbs are the
    # v1→v2 seam: the Rust shell swaps ExecStart, nothing else moves.
    (mkIf cfg.desktop.enable (
      let
        desktopDefaults = import ../desktop/defaults.nix { };
        vogixDesktopQml =
          pkgs.vogix-desktop-qml or (pkgs.callPackage ../../packages/vogix-desktop-qml.nix { });

        # Defaults carry bare-slot shorthand; user tokens arrive complete
        # from the option type. Normalize AFTER the merge so the emitted
        # token shape is always { slot, alpha } — the praxis SlotMapping.
        normalizeToken = t:
          { alpha = 1.0; } // (if builtins.isString t then { slot = t; } else t);
        mergedSurfaces =
          lib.mapAttrs (_: lib.mapAttrs (_: normalizeToken))
            (lib.recursiveUpdate desktopDefaults.surfaces cfg.desktop.surfaces);

        desktopJson = builtins.toJSON {
          schema = 1;
          font = { inherit (cfg.desktop.font) family size; };
          bar = {
            inherit (cfg.desktop.bar) enable position height;
            layout = { inherit (cfg.desktop.bar.layout) left center right; };
          };
          notifications = {
            inherit (cfg.desktop.notifications) enable defaultTimeout maxVisible appRules;
          };
          osd = { inherit (cfg.desktop.osd) enable timeout; };
          polkit = { inherit (cfg.desktop.polkit) enable; };
          lock = { inherit (cfg.desktop.lock) enable pamService; };
          background = { inherit (cfg.desktop.background) enable; };
          idle = { inherit (cfg.desktop.idle) dim lock screenOff suspend; };
          surfaces = mergedSurfaces;
        };
        desktopJsonFile = pkgs.writeText "vogix-desktop.json" desktopJson;
      in
      {
        home.file.".local/state/vogix/desktop.json".source = desktopJsonFile;

        # Registers the QML tree as the `vogix` quickshell config
        # (~/.config/quickshell/vogix → the package), so `qs -c vogix`
        # resolves it. HM's own unit machinery stays off — the unit below is
        # the contract, and its name must survive the v2 swap.
        programs.quickshell = {
          enable = true;
          configs.vogix = lib.mkDefault "${vogixDesktopQml}";
          systemd.enable = false;
        };

        systemd.user.services.vogix-desktop = {
          Unit = {
            Description = "Vogix Desktop Shell (bar, notifications, lock surfaces)";
            After = [ "graphical-session.target" ];
            PartOf = [ "graphical-session.target" ];
            # Qt exits via _exit() on a lost Wayland connection (GPU reset);
            # cap the restart loop like vogix-input does.
            StartLimitBurst = 3;
            StartLimitIntervalSec = 30;
            # A desktop.json-only change reloads in place (sd-switch): the
            # shell re-reads both contract files, MainPID unchanged. The
            # store-symlink swap is invisible to file watchers, which is
            # also why the watcher is disabled outright below.
            X-Reload-Triggers = [ "${desktopJsonFile}" ];
          };

          Service = {
            Type = "simple";
            ExecStart = "${config.programs.quickshell.package}/bin/qs -n -c vogix";
            ExecReload = "${cfg.package}/bin/vogix desktop reload";
            Restart = "on-failure";
            RestartSec = 2;
            Environment = [
              "QS_DISABLE_FILE_WATCHER=1"
              "QS_NO_RELOAD_POPUP=1"
              "RUST_LOG=vogix=${cfg.logLevel}"
            ];
          };

          Install = {
            WantedBy = [ "graphical-session.target" ];
          };
        };

        # The lock hook: systemd-lock-handler (enabled by vogix's NixOS
        # module) translates logind's lock/sleep signals into the user
        # lock.target / sleep.target; this oneshot engages the shell's lock
        # BEFORE either proceeds, and --wait-secure means suspend cannot race
        # ahead of an uncovered output.
        systemd.user.services.vogix-lock = lib.mkIf cfg.desktop.lock.enable {
          Unit = {
            Description = "Vogix session lock (locks before lock.target/sleep.target)";
            PartOf = [ "lock.target" "sleep.target" ];
            Before = [ "lock.target" "sleep.target" ];
          };

          Service = {
            Type = "oneshot";
            ExecStart = "${cfg.package}/bin/vogix desktop lock --wait-secure 4";
          };

          Install = {
            WantedBy = [ "lock.target" "sleep.target" ];
          };
        };
      }
    ))

    # Behavior: generate the hyprland config
    # Note: always active when vogix is enabled (no separate mkIf on behaviorCfg
    # to avoid infinite recursion between config definition and evaluation)
    {
      # Merge defaults into behavior config
      programs.vogix.behavior = {
        keybindings = lib.mkDefault behaviorDefaults.keybindings;
        modes = {
          app = lib.mkDefault behaviorDefaults.modes.app;

          # Derive the app-mode border color from the vogix semantic theme.
          # Modes are NOT statuses — navigation modes use neutral/accent slots,
          # never warning/danger/notice (those are reserved for real conditions).
          # The flat default is a single `app` mode (no CapsLock sub-modes), so
          # only `app` needs a colour.
          modeColors =
            let
              colors = cfg.colors or { };
              toRgb = hex: let h = lib.removePrefix "#" hex; in "rgb(${h})";
            in
            {
              app = {
                active = toRgb (colors.foreground-border or "585b70");
                inactive = toRgb (colors.background-selection or "313244");
              };
            };
        };

        # Generated outputs for downstream consumption
        generatedHyprland = behaviorModule.mkHyprlandConfig behaviorCfg;
      };
    }

    {
      # Install vogix binary
      home.packages = [ cfg.package ];

      # Expose semantic color API for application modules
      programs.vogix.colors = vogix16Lib.semanticColors selectedColors;

      # Create theme symlinks in ~/.local/share/vogix/themes/
      xdg.dataFile = lib.mkMerge [
        # Theme variant symlinks
        (listToAttrs (
          flatten (
            mapAttrsToList
              (
                themeName: variants:
                  mapAttrsToList
                    (
                      variantName: pkg:
                        nameValuePair "vogix/themes/${themeName}-${variantName}" {
                          source = pkg;
                        }
                    )
                    variants
              )
              themeVariantPackages
          )
        ))

      ];

      # Create state directory, config.toml, current-theme symlink, and app config symlinks via activation
      # We use activation instead of xdg.configFile to avoid conflicts with programs.*.enable
      home.activation.vogixSetup = lib.hm.dag.entryAfter [ "writeBoundary" ] ''
        # Create state directory
        $DRY_RUN_CMD mkdir -p "${config.xdg.stateHome}/vogix"

        # Generate config.toml in state directory
        $DRY_RUN_CMD cat > "${config.xdg.stateHome}/vogix/config.toml" << 'VOGIX_CONFIG_EOF'
        ${configToml}
        VOGIX_CONFIG_EOF
        $VERBOSE_ECHO "Generated config.toml at ${config.xdg.stateHome}/vogix/config.toml"

        # Create initial current-theme symlink if it doesn't exist
        _currentLink="${config.xdg.stateHome}/vogix/current-theme"
        _defaultTarget="${config.xdg.dataHome}/vogix/themes/${defaultThemeVariant}"

        if [ ! -L "$_currentLink" ]; then
          $DRY_RUN_CMD ln -sfT "$_defaultTarget" "$_currentLink"
          $VERBOSE_ECHO "Created initial current-theme symlink: $_currentLink -> $_defaultTarget"
        fi

        # Create app config symlinks
        # These point to ~/.local/state/vogix/current-theme/{app}/{config}
        ${concatMapStringsSep "\n" (
          app:
          let
            appModule = appGenerators.${app} or null;
            configFileName = if appModule != null then appModule.configFile or "config" else "config";
            themeFileName = if appModule != null then appModule.themeFile or null else null;
            # Some apps store configs in ~/.local/share instead of ~/.config
            dataDir = if appModule != null then appModule.dataDir or null else null;
            configDir = if dataDir != null
              then "${config.xdg.dataHome}/${dataDir}"
              else "${config.xdg.configHome}/${app}";
            stateDir = "${config.xdg.stateHome}/vogix/current-theme/${app}";
          in
          ''
            # Setup ${app}
            $DRY_RUN_CMD mkdir -p "${configDir}"

            # Config file symlink
            _configFile="${configDir}/${configFileName}"
            _configTarget="${stateDir}/${configFileName}"
            if [ -L "$_configFile" ] || [ ! -e "$_configFile" ]; then
              $DRY_RUN_CMD ln -sfT "$_configTarget" "$_configFile"
              $VERBOSE_ECHO "Created symlink: $_configFile -> $_configTarget"
            fi
            ${optionalString (themeFileName != null) ''

              # Theme file symlink (may be in a subdirectory like themes/vogix.tmTheme)
              _themeFile="${configDir}/${themeFileName}"
              _themeTarget="${stateDir}/${themeFileName}"
              _themeDir="$(dirname "$_themeFile")"
              $DRY_RUN_CMD mkdir -p "$_themeDir"
              if [ -L "$_themeFile" ] || [ ! -e "$_themeFile" ]; then
                $DRY_RUN_CMD ln -sfT "$_themeTarget" "$_themeFile"
                $VERBOSE_ECHO "Created symlink: $_themeFile -> $_themeTarget"
              fi
            ''}
          ''
        ) themedApps}
      '';
    }

    # Apply ANSI 16 colors to terminal via OSC 4 sequences on every interactive shell
    # Reads palette from current-theme/console/palette and sets colors 0-15
    {
      programs.bash.initExtra = mkIf (config.programs.bash.enable or false) ''
        # Vogix: set terminal ANSI colors from theme palette
        _vogix_palette="''${XDG_STATE_HOME:-$HOME/.local/state}/vogix/current-theme/console/palette"
        if [ -f "$_vogix_palette" ]; then
          _i=0
          while IFS= read -r _color; do
            _hex="''${_color#\#}"
            printf '\033]4;%d;#%s\033\\' "$_i" "$_hex"
            _i=$((_i + 1))
          done < "$_vogix_palette"
          # Set background (OSC 11) and foreground (OSC 10) from color 0 and 7
          _bg=$(sed -n '1p' "$_vogix_palette"); _bg="''${_bg#\#}"
          _fg=$(sed -n '8p' "$_vogix_palette"); _fg="''${_fg#\#}"
          printf '\033]10;#%s\033\\' "$_fg"
          printf '\033]11;#%s\033\\' "$_bg"
        fi
        unset _vogix_palette _i _color _hex _bg _fg
      '';

      programs.zsh.initExtra = mkIf (config.programs.zsh.enable or false) ''
        # Vogix: set terminal ANSI colors from theme palette
        _vogix_palette="''${XDG_STATE_HOME:-$HOME/.local/state}/vogix/current-theme/console/palette"
        if [ -f "$_vogix_palette" ]; then
          _i=0
          while IFS= read -r _color; do
            _hex="''${_color#\#}"
            printf '\033]4;%d;#%s\033\\' "$_i" "$_hex"
            _i=$((_i + 1))
          done < "$_vogix_palette"
          _bg=$(sed -n '1p' "$_vogix_palette"); _bg="''${_bg#\#}"
          _fg=$(sed -n '8p' "$_vogix_palette"); _fg="''${_fg#\#}"
          printf '\033]10;#%s\033\\' "$_fg"
          printf '\033]11;#%s\033\\' "$_bg"
        fi
        unset _vogix_palette _i _color _hex _bg _fg
      '';
    }

    # Apply theme on login via shell profile (needs TTY access for console colors)
    # Add to bash profile if bash is enabled
    (mkIf (config.programs.bash.enable or false) {
      programs.bash.profileExtra = ''
        # Apply vogix theme on login (restores theme after reboot)
        ${cfg.package}/bin/vogix theme refresh --quiet 2>/dev/null || true
      '';
    })

    # Add to zsh profile if zsh is enabled
    (mkIf (config.programs.zsh.enable or false) {
      programs.zsh.profileExtra = ''
        # Apply vogix theme on login (restores theme after reboot)
        ${cfg.package}/bin/vogix theme refresh --quiet 2>/dev/null || true
      '';
    })

    # Add to fish profile if fish is enabled
    (mkIf (config.programs.fish.enable or false) {
      programs.fish.loginShellInit = ''
        # Apply vogix theme on login (restores theme after reboot)
        ${cfg.package}/bin/vogix theme refresh --quiet 2>/dev/null; or true
      '';
    })

    # Wezterm keybindings (smart Ctrl+C/V for Super→Ctrl remap)
    (mkIf (config.programs.wezterm.enable or false) {
      programs.wezterm.extraConfig = lib.mkAfter (
        let weztermApp = appGenerators.wezterm or null;
        in optionalString (weztermApp != null && weztermApp ? keybindings) weztermApp.keybindings
      );
    })

    # Optional daemon service for auto-regeneration
    (mkIf cfg.enableDaemon {
      systemd.user.services.vogix-daemon = {
        Unit = {
          Description = "Vogix Theme Management Daemon";
          # Member of the graphical session (NOT default.target). The daemon
          # restores the session and monitors Hyprland events — both need
          # WAYLAND_DISPLAY + HYPRLAND_INSTANCE_SIGNATURE, which are only
          # exported into the systemd-user environment by Hyprland's startup
          # (dbus-update-activation-environment + the hyprland-session.target
          # restart). default.target is reached at *login*, BEFORE that import,
          # so a default.target-wanted daemon captured an empty env, mis-detected
          # an "empty desktop", and auto-restored apps that died with no Wayland
          # display. graphical-session.target is (re)started by Hyprland AFTER
          # the import, so members inherit the full env — like the input engine.
          After = [ "graphical-session.target" ];
          PartOf = [ "graphical-session.target" ];
        };

        Service = {
          Type = "simple";
          ExecStart = "${cfg.package}/bin/vogix daemon";
          Restart = "on-failure";
          RestartSec = 5;
          # Verbosity flows to journald; see programs.vogix.logLevel.
          # VOGIX_AUTO_RESTORE gates boot-time session re-spawn (auto-save is
          # unaffected); see programs.vogix.autoRestoreSession.
          Environment = [
            "RUST_LOG=vogix=${cfg.logLevel}"
            "VOGIX_AUTO_RESTORE=${if cfg.autoRestoreSession then "1" else "0"}"
          ];
        };

        Install = {
          WantedBy = [ "graphical-session.target" ];
        };
      };
    })

    # ── Input engine: render the schema + run the input daemon ──
    # The state file is the bridge from the authored Nix config to the Rust
    # runtime — `Schema::load()` reads it on startup. Writing it via
    # home.file (instead of an activation script) keeps it under nix's GC
    # roots and gives us atomic replacement on switch.
    {
      home.file.".local/state/vogix/input.json" = {
        text = behaviorModule.mkSchemaJSON behaviorCfg;
      };

      # The user systemd service that grabs evdev, runs the praxis-validated
      # mode statechart, and dispatches to Hyprland over its control socket.
      # vogix is the sole input engine; the user is in the `input` + `uinput`
      # groups (set by the NixOS module) so the grab + uinput emit don't need root.
      #
      # `After = graphical-session.target` because the Hyprland IPC socket is
      # owned by that session; dispatches before it lands would be dropped.
      systemd.user.services.vogix-input = {
        Unit = {
          Description = "Vogix Input Engine (ontology-driven keybinding + mode engine)";
          After = [ "graphical-session.target" ];
          PartOf = [ "graphical-session.target" ];
          # Hard cap on the restart loop. The engine grabs evdev as its last
          # startup step; a tight crash-restart loop briefly takes the
          # keyboard away from whoever else has it (login screen, TTY) every
          # cycle. Three failures in 30s → service goes permanently failed
          # and stops restarting, so the user always retains a usable
          # keyboard even if our daemon is fundamentally broken on this host.
          StartLimitBurst = 3;
          StartLimitIntervalSec = 30;
        };

        Service = {
          Type = "simple";
          ExecStart = "${cfg.package}/bin/vogix input run";
          Restart = "on-failure";
          # 2s back-off between attempts within the burst window.
          RestartSec = 2;
          # Verbosity flows to journald; see programs.vogix.logLevel. Raise to
          # `debug` to see every keybinding decision in
          # `journalctl --user -u vogix-input`.
          Environment = [ "RUST_LOG=vogix=${cfg.logLevel}" ];
        };

        Install = {
          WantedBy = [ "graphical-session.target" ];
        };
      };
    }

  ]);
}

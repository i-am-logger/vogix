{
  description = "Vogix - Runtime theme management for NixOS";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    # The desktop shell's QML runtime, pinned to the v0.3.1 tag: nixpkgs still
    # carries 0.3.0, and 0.3.1 fixes "session lock crashes on sleep, wake,
    # DPMS, and unlocking" — a lock-screen must not ride the older build.
    # `follows` is required: upstream warns that quickshell built against a
    # different nixpkgs than its Qt deps crashes. Drop this input when nixpkgs
    # reaches ≥0.3.1.
    quickshell = {
      url = "git+https://git.outfoxxed.me/quickshell/quickshell?ref=refs/tags/v0.3.1";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    home-manager = {
      url = "github:nix-community/home-manager";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    devenv = {
      url = "github:cachix/devenv";
      inputs = {
        nixpkgs.follows = "nixpkgs";
        rust-overlay.follows = "rust-overlay";
      };
    };
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    # Base16/Base24 color schemes - forked with directory-based structure
    # Each theme is a directory with variant files (dark.yaml, light.yaml, etc.)
    tinted-schemes = {
      url = "github:i-am-logger/tinted-schemes";
      flake = false;
    };

    # ANSI 16-color terminal schemes - forked with directory-based structure
    # Uses ansi16/ directory with theme directories containing variant files
    iterm2-schemes = {
      url = "github:i-am-logger/iTerm2-Color-Schemes";
      flake = false;
    };

    # vogix16 design system themes
    # Directory-based structure: {theme}/{variant}.toml (day/night variants)
    vogix16-themes = {
      url = "github:i-am-logger/vogix16-themes";
      flake = false;
    };

    # liquidctl fork with Kraken 2024 Elite RGB ring support
    liquidctl-src = {
      url = "github:i-am-logger/liquidctl/feat/kraken-2024-elite-rgb";
      flake = false;
    };
  };

  nixConfig = {
    extra-trusted-public-keys = "devenv.cachix.org-1:w1cLUi8dv3hnoSPGAuibQv+f9TZLr6cv/Hm9XgU50cw=";
    extra-substituters = "https://devenv.cachix.org";
  };

  outputs =
    { self
    , nixpkgs
    , home-manager
    , ...
    }@inputs:
    let
      systems = [
        "x86_64-linux"
        "aarch64-linux"
      ];
      forAllSystems = nixpkgs.lib.genAttrs systems;
      # Every vogix-licensed (CC BY-NC-SA 4.0) package a pkgs instantiation
      # may evaluate — ONE list, used by every allowUnfreePredicate here and
      # mirrored by consumers (mynixos my.system.allowedUnfreePackages).
      unfreePackageNames = [ "vogix" "vogix-desktop-qml" "vogix-sddm-theme" "vogix-plymouth" ];
    in
    {
      # NixOS module (console colors, security wrappers, hardware)
      nixosModules.default = import ./nix/modules/nixos.nix {
        liquidctlSrc = inputs.liquidctl-src;
      };

      # Home Manager module
      # Pass scheme sources for theme import
      homeManagerModules.default = import ./nix/modules/home-manager {
        tintedSchemes = inputs.tinted-schemes;
        iterm2Schemes = inputs.iterm2-schemes;
        vogix16Themes = inputs.vogix16-themes;
      };

      # Packages for each system - from devenv outputs
      packages = forAllSystems (
        system:
        let
          pkgs = import nixpkgs {
            inherit system;
            config.allowUnfreePredicate = pkg: builtins.elem (nixpkgs.lib.getName pkg) unfreePackageNames;
          };
          # Built with nixpkgs' standard buildRustPackage (nix/packages/vogix.nix).
          # cargo's vendoring fetches the whole praxis repo, so its
          # `version.workspace = true` resolves against the workspace root — which
          # is why this replaced the devenv/crate2nix build (crate2nix's per-crate
          # vendor could not find a workspace root for the inherited version).
          vogix-unwrapped = pkgs.callPackage ./nix/packages/vogix.nix { };
          # Wrap to add shell completions (kept out of the build derivation).
          vogix = pkgs.runCommand "vogix-${vogix-unwrapped.version}"
            {
              nativeBuildInputs = [ pkgs.installShellFiles ];
              inherit (vogix-unwrapped) meta;
            } ''
            cp -r ${vogix-unwrapped} $out
            chmod -R u+w $out
            installShellCompletion --cmd vogix \
              --bash <($out/bin/vogix completions bash) \
              --zsh <($out/bin/vogix completions zsh) \
              --fish <($out/bin/vogix completions fish)
          '';
          # The desktop shell's v1 QML tree (a quickshell config directory).
          vogix-desktop-qml = pkgs.callPackage ./nix/packages/vogix-desktop-qml.nix { };
          # The session locker, shaped as its own package so it slots into
          # environment.locker / $LOCKER selectors: engage the shell's lock
          # and fail unless the compositor reports it SECURE within 4s.
          vogix-lock = pkgs.writeShellApplication {
            name = "vogix-lock";
            runtimeInputs = [ vogix ];
            text = ''exec vogix desktop lock --wait-secure 4 "$@"'';
          };
          # The launcher, shaped as its own package so it slots into
          # environment.launcher / $LAUNCHER selectors. Speaks the
          # walker-compatible `--dmenu [-p PROMPT]` picker form (items on
          # stdin, choice on stdout, exit 1 on cancel) so `vogix input keys`
          # and scripts keep a picker whichever launcher the host selects.
          vogix-launcher = pkgs.writeShellApplication {
            name = "vogix-launcher";
            runtimeInputs = [ vogix ];
            text = ''
              if [ "''${1:-}" = "--dmenu" ]; then
                shift
                prompt=""
                while [ $# -gt 0 ]; do
                  case "$1" in
                    -p)
                      prompt="''${2:-}"
                      shift
                      if [ $# -gt 0 ]; then shift; fi
                      ;;
                    *) shift ;;
                  esac
                done
                if [ -n "$prompt" ]; then
                  exec vogix desktop select --prompt "$prompt"
                fi
                exec vogix desktop select
              fi
              exec vogix desktop launcher "$@"
            '';
          };
          # The SDDM greeter theme with its neutral fallback palette; the
          # NixOS module rebuilds it with the real palette via callPackage.
          vogix-sddm-theme = pkgs.callPackage ./nix/packages/vogix-sddm-theme.nix { };
        in
        {
          inherit vogix vogix-desktop-qml vogix-lock vogix-launcher vogix-sddm-theme;
          # The OpenRGB server vogix.openrgb runs (nix/packages/openrgb.nix).
          openrgb = self.lib.openrgbPatched pkgs;
          default = vogix;
        }
      );

      # OpenRGB built from vogix's OpenRGB source with a host's own package
      # set: the server package vogix.openrgb defaults to, for hosts that
      # set services.hardware.openrgb.package themselves
      # (`services.hardware.openrgb.package = vogix.lib.openrgbPatched pkgs;`).
      lib.openrgbPatched = pkgs: pkgs.callPackage ./nix/packages/openrgb.nix { };

      # Overlay to make vogix (and the desktop shell's QML runtime) available
      # in pkgs. quickshell's own overlay is COMPOSED — its package is built
      # against the consumer's final pkgs, never re-exported as a foreign
      # instance, so its Qt deps always match the host nixpkgs (upstream warns
      # a mismatch crashes).
      overlays.default = final: prev:
        (inputs.quickshell.overlays.default final prev) // {
          inherit (self.packages.${prev.stdenv.hostPlatform.system}) vogix vogix-desktop-qml vogix-lock vogix-launcher vogix-sddm-theme;
        };

      # Liquidctl overlay (patched fork with Kraken 2024 Elite RGB ring support)
      overlays.liquidctl = _final: prev: {
        liquidctl = prev.liquidctl.overridePythonAttrs (_old: {
          src = inputs.liquidctl-src;
        });
      };

      # NixOS VM for testing
      nixosConfigurations.vogix-test-vm = nixpkgs.lib.nixosSystem {
        system = "x86_64-linux";
        modules = [
          ./nix/vm/test-vm.nix
          self.nixosModules.default
          home-manager.nixosModules.home-manager
          {
            # Make vogix package available in pkgs via overlay
            nixpkgs.overlays = [ self.overlays.default ];

            # Allow unfree license for testing
            nixpkgs.config.allowUnfreePredicate = pkg: builtins.elem (nixpkgs.lib.getName pkg) unfreePackageNames;

            home-manager.useGlobalPkgs = true;
            home-manager.useUserPackages = true;
            home-manager.users.vogix = import ./nix/vm/home.nix;
            home-manager.sharedModules = [ self.homeManagerModules.default ];
          }
        ];
      };

      # Automated integration tests - split by feature area
      # Run individual tests: nix build .#checks.x86_64-linux.smoke
      # Run all tests: nix flake check
      checks = forAllSystems (
        system:
        let
          pkgs = import nixpkgs {
            inherit system;
            config.allowUnfreePredicate = pkg: builtins.elem (nixpkgs.lib.getName pkg) unfreePackageNames;
          };
          testArgs = {
            inherit pkgs home-manager self;
            vogix16Themes = inputs.vogix16-themes;
          };
          machineContract = import ./nix/checks/machine-contract.nix {
            inherit pkgs nixpkgs home-manager self unfreePackageNames;
            # The theme sources the home-manager module renders into
            # config.toml's [theme_sources].
            schemeSources = {
              vogix16 = inputs.vogix16-themes;
              base16 = "${inputs.tinted-schemes}/base16";
              base24 = "${inputs.tinted-schemes}/base24";
              ansi16 = "${inputs.iterm2-schemes}/ansi16";
            };
          };
        in
        {
          # Pure-Nix unit tests for the config generators (appearance,
          # behavior, the merged Hyprland render incl. the Lua projection).
          # Each suite deepSeq-forces its assertions and THROWS on failure, so
          # a broken generator fails this derivation at INSTANTIATION — it
          # runs under `nix flake check --no-build` and never needs a builder.
          nix-unit =
            let
              inherit (pkgs) lib;
              suites = {
                appearance = (import ./nix/modules/appearance/tests.nix { inherit pkgs lib; }).passed;
                behavior = (import ./nix/modules/behavior/tests.nix { inherit pkgs lib; }).passed;
                hyprland = (import ./nix/modules/hyprland-tests.nix { inherit pkgs lib; }).passed;
                contract = (import ./nix/modules/contract-tests.nix { inherit pkgs lib; }).passed;
              };
            in
            pkgs.runCommand "vogix-nix-unit"
              (builtins.mapAttrs (_: toString) suites)
              ''
                echo "appearance: $appearance  behavior: $behavior  hyprland: $hyprland  contract: $contract"
                touch $out
              '';

          # The appearance options must actually REACH the rendered Hyprland
          # config through the module system. They used to be declared behind
          # an `options = { … }` wrapper that the merge site never unwrapped,
          # so every `programs.vogix.appearance.{gaps,decoration,blur,…}`
          # setting was silently discarded and the render always used
          # defaults.nix. This evaluates a real home-manager configuration and
          # asserts a non-default gap survives into the config text.
          appearance-options =
            let
              hmConf = home-manager.lib.homeManagerConfiguration {
                inherit pkgs;
                modules = [
                  self.homeManagerModules.default
                  {
                    home = {
                      username = "t";
                      homeDirectory = "/home/t";
                      stateVersion = "24.11";
                    };
                    programs.vogix = {
                      enable = true;
                      appearance = {
                        theme = "yoga";
                        variant = "night";
                        prebuiltThemes = [ "yoga" ];
                        gaps.inner = 33;
                        decoration.rounding = 4;
                      };
                      enableDaemon = false;
                    };
                    wayland.windowManager.hyprland = {
                      enable = true;
                      package = null;
                      portalPackage = null;
                    };
                  }
                ];
              };
              inherit (hmConf.config.xdg.configFile."hypr/hyprland.conf") text;
              ok = pkgs.lib.hasInfix "gaps_in=33" text && pkgs.lib.hasInfix "rounding=4" text;
            in
            assert ok || throw "programs.vogix.appearance.* did not reach the rendered Hyprland config";
            pkgs.runCommand "vogix-appearance-options" { } ''
              echo "appearance options reach the rendered config"
              touch $out
            '';

          # Login shells run nothing from vogix; the theme is restored once
          # per graphical session by a user oneshot. Evaluated for a profile
          # with bash, zsh and fish enabled: no login-time shell text names
          # the vogix binary; vogix-theme-restore is a oneshot without
          # RemainAfterExit, wanted by and ordered after
          # graphical-session.target, running the profile's
          # `vogix theme refresh` at its log level; config.toml carries the
          # apply hooks (the greeter's included) as [hooks."<name>"] tables
          # and no [hardware.*] table. Checked at instantiation, so
          # `--no-build` covers it.
          login-profile =
            let
              inherit (pkgs) lib;
              inherit (home-manager.lib.homeManagerConfiguration {
                inherit pkgs;
                modules = [
                  self.homeManagerModules.default
                  {
                    home = {
                      username = "t";
                      homeDirectory = "/home/t";
                      stateVersion = "24.11";
                    };
                    programs = {
                      bash.enable = true;
                      zsh.enable = true;
                      fish.enable = true;
                      vogix = {
                        enable = true;
                        appearance = {
                          theme = "yoga";
                          variant = "night";
                          prebuiltThemes = [ "yoga" ];
                        };
                        logLevel = "debug";
                        greeter.sync = true;
                        themeApply.probe = "printf %s {{base01}}";
                        enableDaemon = false;
                      };
                    };
                  }
                ];
              }) config;
              loginTexts = {
                "programs.bash.profileExtra" = config.programs.bash.profileExtra;
                "programs.zsh.profileExtra" = config.programs.zsh.profileExtra;
                "programs.zsh.loginExtra" = config.programs.zsh.loginExtra;
                "programs.fish.loginShellInit" = config.programs.fish.loginShellInit;
              };
              runningVogix = builtins.attrNames (lib.filterAttrs (_: lib.hasInfix "bin/vogix") loginTexts);
              unit = config.systemd.user.services.vogix-theme-restore or null;
              refresh = "${config.programs.vogix.package}/bin/vogix theme refresh";
              unitOk = unit != null
                && unit.Service.Type == "oneshot"
                && !(unit.Service ? RemainAfterExit)
                && lib.toList unit.Service.ExecStart == [ refresh ]
                && builtins.elem "RUST_LOG=vogix=debug" (lib.toList unit.Service.Environment)
                && builtins.elem "graphical-session.target" unit.Unit.After
                && builtins.elem "graphical-session.target" unit.Install.WantedBy;
              setup = config.home.activation.vogixSetup.data;
              hooksOk = lib.hasInfix ''[hooks."greeter"]'' setup
                && lib.hasInfix ''
                [hooks."probe"]
                command = """printf %s {{base01}}"""''
                setup
                && !(lib.hasInfix "[hardware" setup);
            in
            assert runningVogix == [ ] || throw "login shells run vogix from ${lib.concatStringsSep ", " runningVogix}";
            assert unitOk || throw "vogix-theme-restore is not a oneshot without RemainAfterExit, wanted by and after graphical-session.target, running `${refresh}` at RUST_LOG=vogix=debug: ${builtins.toJSON unit}";
            assert hooksOk || throw "config.toml does not carry the apply hooks as [hooks.\"<name>\"] tables, or still has a [hardware] table";
            pkgs.runCommand "vogix-login-profile" { } ''
              echo "login shells run nothing from vogix; vogix-theme-restore restores each graphical session"
              touch $out
            '';

          # desktop.json is the v1→v2 contract, so its DEFAULT rendering is
          # pinned byte-for-byte (key-sorted): schema drift must arrive as a
          # deliberate, reviewed edit of the pin file, never as a side effect
          # of an option refactor.
          desktop-options =
            let
              hmConf = home-manager.lib.homeManagerConfiguration {
                inherit pkgs;
                modules = [
                  self.homeManagerModules.default
                  {
                    home = {
                      username = "t";
                      homeDirectory = "/home/t";
                      stateVersion = "24.11";
                    };
                    programs.vogix = {
                      enable = true;
                      appearance = {
                        theme = "yoga";
                        variant = "night";
                        prebuiltThemes = [ "yoga" ];
                      };
                      desktop.enable = true;
                      enableDaemon = false;
                    };
                    wayland.windowManager.hyprland = {
                      enable = true;
                      package = null;
                      portalPackage = null;
                    };
                  }
                ];
              };
              rendered = hmConf.config.home.file.".local/state/vogix/desktop.json".source;
              # Custom cells: a defined one renders whole into desktop.json;
              # placing an undefined one, or naming one with a `/`, fails.
              customConf = desktop: home-manager.lib.homeManagerConfiguration {
                inherit pkgs;
                modules = [
                  self.homeManagerModules.default
                  {
                    home = {
                      username = "t";
                      homeDirectory = "/home/t";
                      stateVersion = "24.11";
                    };
                    programs.vogix = {
                      enable = true;
                      appearance = {
                        theme = "yoga";
                        variant = "night";
                        prebuiltThemes = [ "yoga" ];
                      };
                      desktop = { enable = true; } // desktop;
                      enableDaemon = false;
                    };
                  }
                ];
              };
              customRendered = (customConf {
                custom.updates = { command = "checkupdates | wc -l"; interval = 3600; };
                bars.right.layout.end = [ "custom/updates" ];
              }).config.home.file.".local/state/vogix/desktop.json".source;
              customRejected = desktop: !(builtins.tryEval (customConf desktop).config.home.username).success;
              # `vogix desktop check` runs while desktop.json builds (the
              # default one above included): a document it rejects, here a
              # menu command the CLI cannot parse, fails that build.
              uncheckable = pkgs.testers.testBuildFailure (customConf {
                launcher.menu = [{ id = "probe"; label = "Probe"; action = "vogix desktop remind add 'Reminder' 10m"; }];
              }).config.home.file.".local/state/vogix/desktop.json".source;
              customGuarded =
                customRejected { bars.top.layout.end = [ "custom/nope" ]; }
                && customRejected { custom."a/b".command = "date"; };
              registry = import ./nix/modules/desktop/registry.nix;
              # The layout options' values, forced: a placement their type
              # rejects fails here, as it fails every build that renders
              # desktop.json from them.
              layoutEvaluates = desktop:
                (builtins.tryEval (builtins.deepSeq (customConf desktop).config.programs.vogix.desktop.bars true)).success;
              # A layout name is typed by the shell's widget registry: every
              # name the registry lets a bar orientation place evaluates on
              # both bars of it, and a name outside the registry does not.
              registryTyped =
                layoutEvaluates
                  {
                    bars = {
                      top.layout.center = registry.placeable.horizontal;
                      bottom.layout.center = registry.placeable.horizontal;
                      left.layout.center = registry.placeable.vertical;
                      right.layout.center = registry.placeable.vertical;
                    };
                  }
                && !layoutEvaluates { bars.top.layout.end = [ "clokc" ]; };
              # ... and by the bar's orientation: on every edge, a section's
              # element type takes a registry name exactly when the
              # registry's `orientation` for it is absent or that edge's. A
              # rail meter on a horizontal bar, and a wide cell on a rail,
              # fail the whole configuration.
              orientationTyped =
                let
                  inherit (hmConf.options.programs.vogix.desktop.type.getSubOptions [ ]) bars;
                  takes = edge: bars.${edge}.layout.start.type.nestedTypes.elemType.check;
                  expected = edge: name: builtins.elem (registry.widgets.${name}.orientation or null) [ null (registry.edgeOrientation edge) ];
                in
                builtins.all (edge: builtins.all (name: takes edge name == expected edge name) registry.names)
                  [ "top" "bottom" "left" "right" ]
                && !layoutEvaluates { bars.left.layout.start = [ "window" ]; }
                && !layoutEvaluates { bars.right.layout.center = [ "oscilloscope" ]; }
                && !layoutEvaluates { bars.top.layout.center = [ "vu-rail" ]; };
              # `vogix desktop check` holds a document that never went
              # through these options to the same registry facts: a rail
              # meter on the top bar and the oscilloscope on the right rail
              # fail the check, and with it the build.
              misplaced = pkgs.testers.testBuildFailure (pkgs.runCommand "vogix-desktop-misplaced"
                { nativeBuildInputs = [ pkgs.jq self.packages.${system}.vogix ]; } ''
                jq '.bars.top.layout.center += ["vu-rail"] | .bars.right.layout.center += ["oscilloscope"]' \
                  ${./nix/modules/desktop/desktop-json.pin.json} > desktop.json
                HOME=$TMPDIR vogix desktop check --config desktop.json
                touch $out
              '');
              unit = hmConf.config.systemd.user.services.vogix-desktop;
              # Exit status 75 is the shell's fresh-start request
              # (desktop/Services/NetworkBackend.qml); the unit must answer
              # it with a restart, and not log it as a failure.
              restartsOn75 = unit.Service.RestartForceExitStatus == 75 && unit.Service.SuccessExitStatus == 75;
              # The shell unit waits for a PipeWire that is not up yet
              # instead of losing it for the whole session.
              waitsForPipewire = builtins.elem "QS_PIPEWIRE_IMMEDIATE_RECONNECT=1" unit.Service.Environment
                && builtins.elem "pipewire.service" unit.Unit.After;
              # By default quickshell produces no DEBUG records at all
              # (desktop.detailedLogs = false).
              sparseLogs = builtins.any (pkgs.lib.hasSuffix " --no-detailed-logs") (pkgs.lib.toList unit.Service.ExecStart);
            in
            assert customGuarded || throw "an undefined custom/<name> placement or a bad custom cell name did not trip its assertion";
            assert registryTyped || throw "the bar layout options do not take exactly the shell's widget registry names";
            assert orientationTyped || throw "a bar's layout options take a widget the registry confines to the other bar orientation, or refuse one it permits";
            assert restartsOn75 || throw "vogix-desktop.service does not restart the shell on exit status 75";
            assert waitsForPipewire || throw "vogix-desktop.service lost its PipeWire ordering or QS_PIPEWIRE_IMMEDIATE_RECONNECT";
            assert sparseLogs || throw "vogix-desktop.service runs quickshell with detailed logs by default";
            pkgs.runCommand "vogix-desktop-options" { nativeBuildInputs = [ pkgs.jq ]; } ''
              jq -S . ${rendered} > got.json
              jq -S . ${./nix/modules/desktop/desktop-json.pin.json} > want.json
              if ! diff -u want.json got.json; then
                echo "the default desktop.json drifted from nix/modules/desktop/desktop-json.pin.json;"
                echo "if the schema change is intended, update the pin in the same commit."
                exit 1
              fi
              jq -e '.custom.updates == {
                  title: "updates", command: "checkupdates | wc -l", output: "text",
                  interval: 3600, watch: [], stream: false, onClick: null, widest: null
                } and .bars.right.layout.end == ["custom/updates"]' ${customRendered}
              grep -F "launcher.menu.probe.action: \`vogix desktop remind add 'Reminder' 10m\` is not a valid vogix command" \
                ${uncheckable}/testBuildFailure.log
              grep -F "bars.top.layout.center: 'vu-rail' is vertical-only and cannot render on a horizontal bar" \
                ${misplaced}/testBuildFailure.log
              grep -F "bars.right.layout.center: 'oscilloscope' is horizontal-only and cannot render on a vertical bar" \
                ${misplaced}/testBuildFailure.log
              touch $out
            '';

          # The shell's runtime is declared, not assumed. Every program the
          # QML spawns by name (the argv[0] of a Process command or an
          # execDetached call) must resolve through the vogix-desktop
          # unit's own PATH wrapper with nothing else on PATH, unless it is
          # a base-system tool or a client of a host daemon that must match
          # that daemon. And the NixOS module turns on the D-Bus services
          # the shell reads while a user runs it — power-profiles-daemon
          # only where no other power manager owns the role.
          desktop-runtime =
            let
              inherit (pkgs) lib;
              hmUser = desktop: {
                imports = [ self.homeManagerModules.default ];
                home = {
                  username = "t";
                  homeDirectory = "/home/t";
                  stateVersion = "24.11";
                };
                programs.vogix = {
                  enable = true;
                  appearance = {
                    theme = "yoga";
                    variant = "night";
                    prebuiltThemes = [ "yoga" ];
                  };
                  desktop.enable = desktop;
                  enableDaemon = false;
                };
              };
              hmConf = home-manager.lib.homeManagerConfiguration {
                inherit pkgs;
                modules = [ (hmUser true) ];
              };
              execStart = builtins.head (lib.toList hmConf.config.systemd.user.services.vogix-desktop.Service.ExecStart);
              desktopEnv = builtins.head (lib.splitString " " execStart);
              host = { desktop, extra ? { } }:
                (nixpkgs.lib.nixosSystem {
                  modules = [
                    self.nixosModules.default
                    home-manager.nixosModules.home-manager
                    extra
                    {
                      nixpkgs.hostPlatform = system;
                      vogix.enable = true;
                      users.users.t.isNormalUser = true;
                      home-manager.users.t = hmUser desktop;
                      system.stateVersion = "24.11";
                    }
                  ];
                }).config.services;
              withShell = host { desktop = true; };
              withShellAndTlp = host { desktop = true; extra.services.tlp.enable = true; };
              withoutShell = host { desktop = false; };
              servicesOk =
                withShell.upower.enable && withShell.power-profiles-daemon.enable
                && withShellAndTlp.upower.enable && !withShellAndTlp.power-profiles-daemon.enable
                && !withoutShell.upower.enable && !withoutShell.power-profiles-daemon.enable;
            in
            assert servicesOk || throw "the NixOS module does not enable UPower/power-profiles-daemon exactly while a user runs the desktop shell";
            pkgs.runCommand "vogix-desktop-runtime"
              { qml = self.packages.${system}.vogix-desktop-qml; } ''
              # Base-system tools, and clients of host daemons (Hyprland,
              # Tailscale, systemd, the NVIDIA driver) that must match the
              # running daemon or driver.
              hostProvided=" sh bash readlink uname pkill systemctl hyprctl tailscale nvidia-smi "
              # Tools the argv[0] scan below cannot see: run inside `sh -c`
              # lines, or from an argv computed at runtime (wttrbar). Each
              # must still occur in the QML, so this list cannot outlive
              # its use.
              embedded="cava wl-copy hyprsunset wttrbar"
              for name in $embedded; do
                grep -rqw -- "$name" "$qml" || { echo "'$name' no longer occurs in the QML; drop it here"; exit 1; }
              done
              names=$(grep -rhoE '(command *[:=] *|execDetached\()\["[A-Za-z0-9_.+-]+"' "$qml" \
                | sed -E 's/.*\["//; s/"$//' | sort -u)
              test -n "$names"
              for name in $names $embedded; do
                case "$hostProvided" in *" $name "*) continue ;; esac
                if ! PATH=/var/empty ${desktopEnv} ${pkgs.runtimeShell} -c "command -v $name" >/dev/null; then
                  echo "the shell spawns '$name', which the vogix-desktop unit's PATH does not provide"
                  exit 1
                fi
                echo "$name: provided"
              done
              touch $out
            '';

          # Lint the desktop shell's QML against the pinned quickshell's
          # modules. Two categories are disabled because quickshell's
          # published qmltypes cannot express them (PanelWindow is creatable
          # and `margins` is a real grouped property — both verified against
          # the 0.3.1 sources); everything else must be clean. Unused imports,
          # which qmllint only mentions by default, are raised to warnings so
          # they fail here too.
          desktop-qmllint =
            let
              qsPkgs = import nixpkgs {
                inherit system;
                overlays = [ self.overlays.default ];
                config.allowUnfreePredicate = pkg: builtins.elem (nixpkgs.lib.getName pkg) unfreePackageNames;
              };
            in
            pkgs.runCommand "vogix-desktop-qmllint"
              {
                nativeBuildInputs = [ pkgs.qt6.qtdeclarative ];
                qml = qsPkgs.vogix-desktop-qml;
                qsQml = "${qsPkgs.quickshell}/lib/qt-6/qml";
                mmQml = "${pkgs.qt6.qtmultimedia}/lib/qt-6/qml";
              } ''
              # quickshell maps the `qs.` module URI onto the config root at
              # runtime; give qmllint the same view with a staged import root
              # where qs/ IS the package.
              mkdir lintroot
              ln -s "$qml" lintroot/qs
              find $qml -name '*.qml' -print0 | xargs -0 qmllint \
                -I "$qsQml" -I "$mmQml" -I "${pkgs.qt6.qtdeclarative}/lib/qt-6/qml" -I "$PWD/lintroot" \
                --unused-imports warning --uncreatable-type disable --unresolved-type disable \
                2>&1 | tee lint.out || true
              if grep -E 'Warning|Error' lint.out | grep -v 'grouped property scope margins'; then
                echo "qmllint found real issues"; exit 1
              fi
              # Flight Deck: square corners on every surface. A radius only
              # ever makes a circle (width / 2), never a rounded corner.
              if grep -rnE --include='*.qml' '^\s*radius:' "$qml" | grep -vE 'radius: *(width|height) / 2$'; then
                echo "rounded corners in the shell QML: the Flight Deck rule is radius 0"; exit 1
              fi
              # A raw dispatch string speaks one config dialect: hyprlang's
              # `workspace 2` is a Lua syntax error under the Lua engine.
              # Compositor writes go through quickshell's typed methods
              # (HyprlandWorkspace.activate()), which pick the dialect.
              if grep -rn --include='*.qml' 'Hyprland\.dispatch(' $qml; then
                echo "raw Hyprland.dispatch() in the shell: use the typed, dialect-aware method"; exit 1
              fi
              # A bar widget opens its panel beside its own bar through its
              # BarAxis (togglePanel); Panels.toggle alone places the panel
              # as a verb-opened one, away from the widget.
              unrouted=$(grep -rl --include='*.qml' 'Panels\.toggle(' "$qml/Bar" | xargs -r grep -L 'togglePanel' || true)
              if [ -n "$unrouted" ]; then
                echo "bar widgets opening a panel without their BarAxis:"; echo "$unrouted"; exit 1
              fi
              # A platform menu's display(window, x, y) takes raw window
              # coordinates; menus open through QsMenuAnchor, which maps its
              # anchor item into window space and hands the compositor's
              # positioner the edge to open from.
              if grep -rn --include='*.qml' '\.display(' $qml; then
                echo "menu .display() in the shell: open menus through QsMenuAnchor"; exit 1
              fi
              touch $out
            '';

          # The audio taps against a real PipeWire daemon: waiting for it,
          # relaunch after an exit, stop and return across a PipeWire
          # restart (nix/checks/desktop-taps.nix).
          desktop-taps = import ./nix/checks/desktop-taps.nix {
            inherit pkgs;
            qsPkgs = import nixpkgs {
              inherit system;
              overlays = [ self.overlays.default ];
              config.allowUnfreePredicate = pkg: builtins.elem (nixpkgs.lib.getName pkg) unfreePackageNames;
            };
          };

          # The shell actually RUNS, as its unit starts it, under a
          # headless cage compositor (nix/checks/desktop-smoke.nix).
          desktop-smoke = import ./nix/checks/desktop-smoke.nix {
            inherit pkgs home-manager;
            hmModule = self.homeManagerModules.default;
            qsPkgs = import nixpkgs {
              inherit system;
              overlays = [ self.overlays.default ];
              config.allowUnfreePredicate = pkg: builtins.elem (nixpkgs.lib.getName pkg) unfreePackageNames;
            };
          };

          # Every desktop theme variant ships its backgrounds: the generated
          # "veil" is always present (rendered from that variant's own
          # palette), curated extras merge in through
          # appearance.extraBackgrounds, and backgrounds.json lists them all —
          # BESIDE theme.json, which must stay byte-identical between render
          # layers and therefore cannot carry store paths.
          desktop-backgrounds =
            let
              hmConf = home-manager.lib.homeManagerConfiguration {
                inherit pkgs;
                modules = [
                  self.homeManagerModules.default
                  {
                    home = {
                      username = "t";
                      homeDirectory = "/home/t";
                      stateVersion = "24.11";
                    };
                    programs.vogix = {
                      enable = true;
                      desktop.enable = true;
                      appearance = {
                        theme = "yoga";
                        variant = "night";
                        prebuiltThemes = [ "yoga" ];
                        extraBackgrounds.yoga.night = [{
                          kind = "image";
                          name = "extra.png";
                          # A copy of one file, not a path into this flake's
                          # source, whose string would carry the whole tree.
                          path = builtins.path { path = ./README.md; name = "extra.png"; };
                        }];
                      };
                      enableDaemon = false;
                    };
                  }
                ];
              };
            in
            pkgs.runCommand "vogix-desktop-backgrounds"
              {
                themePkg = hmConf.config.xdg.dataFile."vogix/themes/yoga-night".source;
                qmlPkg = self.packages.${system}.vogix-desktop-qml;
                nativeBuildInputs = [ pkgs.jq ];
              } ''
              test -f "$themePkg/vogix-desktop/backgrounds.json"
              test -e "$themePkg/vogix-desktop/backgrounds/veil"
              jq -e '.backgrounds[0].kind == "generated" and (.backgrounds | length) == 3' \
                "$themePkg/vogix-desktop/backgrounds.json"
              jq -e '.backgrounds[1].kind == "shader" and .backgrounds[1].name == "aurora"' \
                "$themePkg/vogix-desktop/backgrounds.json"
              jq -e '.backgrounds[2].name == "extra.png"' "$themePkg/vogix-desktop/backgrounds.json"
              # The shell ships the shader precompiled for the RHI backends.
              test -f "$qmlPkg/data/aurora.frag.qsb"
              echo "backgrounds present, generated first, aurora shader second, extras merged"
              touch $out
            '';

          # The shell's pure logic (Services/lib parsers and policies, the
          # quickshell-free QML types) under Qt Quick Test, and its probe
          # scripts against fixture trees.
          desktop-logic = import ./tests/desktop {
            inherit pkgs;
            qml = self.packages.${system}.vogix-desktop-qml;
          };

          # Quick sanity checks (binary, status, list, systemd)
          smoke = import ./nix/vm/tests/smoke.nix testArgs;

          # Symlinks, runtime dirs, config structure
          architecture = import ./nix/vm/tests/architecture.nix testArgs;

          # Theme/variant switching with config verification
          theme-switching = import ./nix/vm/tests/theme-switching.nix testArgs;

          # Cross-scheme tests, palette format validation
          scheme-switching = import ./nix/vm/tests/scheme-switching.nix testArgs;

          # Darker/lighter navigation, catppuccin multi-variant
          navigation = import ./nix/vm/tests/navigation.nix testArgs;

          # Combined flags, list options, error handling
          cli = import ./nix/vm/tests/cli.nix testArgs;

          # State persistence, consistency
          state = import ./nix/vm/tests/state.nix testArgs;

          # Session save/restore/undo
          session = import ./nix/vm/tests/session.nix testArgs;

          # Runtime size inspection
          runtime-size = import ./nix/vm/tests/runtime-size.nix testArgs;

          # Rapid switching tests
          stress = import ./nix/vm/tests/stress.nix testArgs;

          # Template architecture tests
          templates = import ./nix/vm/tests/templates.nix testArgs;

          # Input ENGINE end-to-end (vogix is the sole input engine): runs the
          # real `vogix input run` against a virtual keyboard + a mock compositor
          # socket and asserts the full daily-driver UX — re-emit/typing, the
          # Super→Ctrl remap, caps tap-sticky / hold-momentary, sub-mode routing,
          # exitAfter, the Esc safety-net, repeat, the single-instance guard, and
          # input-locks.json following the grabbed keyboard's LEDs.
          input-engine = import ./nix/vm/tests/input-engine.nix testArgs;

          # The desktop shell on a real Hyprland 0.56 session (Lua and
          # hyprlang config providers) with PipeWire, NetworkManager and
          # the input engine: clicks, placement, tray menus, privacy, the
          # LANG cell, taps across a PipeWire restart, lock-time sampling
          # and the late-NetworkManager restart.
          desktop-hyprland = import ./nix/vm/tests/desktop-hyprland.nix testArgs;

          # openrgb.service as vogix.openrgb runs it, against the real
          # OpenRGB server: Type=notify readiness (listening when a restart
          # returns), the settings merge into OpenRGB.json, and stock
          # nixpkgs OpenRGB failing that unit's start; the module wiring
          # and its readiness assertion are checked at instantiation.
          openrgb-readiness = import ./nix/vm/tests/openrgb-readiness.nix { inherit pkgs self; };

          # The NixOS machine module at evaluation (nix/checks/machine-contract.nix):
          # machine.json for the dram-rgb, keychron-k2-he and kraken-elite
          # modules, the owner units and when each exists, the machine
          # owner's default and the console colours following it, which
          # user's console app reloads the VT palette, and the
          # declarations the module refuses (an OpenRGB server without
          # readiness, a command outside the store, a non-vogix owner, the
          # removed vogix.hardware.themeApply, ill-typed devices).
          machine-contract = machineContract.contract;

          # `vogix machine validate`, the owner units' own loader, accepts
          # that rendered machine.json and a palette the owner's CLI
          # published, and refuses each with a field the schema lacks.
          machine-contract-validate = machineContract.validate;

          # The VT palette the system is built with is the one the runtime
          # applies, for a theme of each scheme: the machine owner's
          # console.colors, their theme package's console/palette and
          # `vogix theme refresh`'s render of templates/<scheme>/console.palette.vogix
          # (nix/checks/machine-contract.nix).
          console-palette = machineContract.console;

          # The machine surfaces as the NixOS module declares them, fed by
          # the declared owner's real CLI: vogix-machine (the kernel VT
          # palette, sysfs checked with no VT switch; a command device re-run
          # on a hidraw hot-add) and vogix-openrgb against the real OpenRGB
          # server with Debug, DDP and Govee devices (colours observed on
          # OpenRGB's own DDP and Govee wire output, confirmed at protocol 6
          # and sent at protocol 5 after a switch); theme changes, another
          # user's apply, a planted palette, SIGHUP and the resume unit,
          # openrgb restarts and a killed server, and a reboot that applies
          # the palette before systemd-user-sessions. `vogix machine inspect`
          # captures the raw controller payloads into $out/capture and the
          # published palette into $out/published (tests/fixtures).
          machine-release = import ./nix/vm/tests/machine-release.nix testArgs;
        }
      );

      # Development shells - using devenv
      # Note: Use 'devenv shell' for development instead of 'nix develop'
      # (devShells kept commented out; the dev environment lives in devenv.nix)
      # devShells = forAllSystems (system:
      #   let
      #     pkgs = import nixpkgs {
      #       inherit system;
      #       config.allowUnfreePredicate = pkg: builtins.elem (nixpkgs.lib.getName pkg) unfreePackageNames;
      #     };
      #   in
      #   {
      #     default = devenv.lib.mkShell {
      #       inherit inputs pkgs;
      #       modules = [ ./devenv.nix ];
      #     };
      #   }
      # );

      # Apps for easy access
      apps = forAllSystems (
        system:
        let
          pkgs = nixpkgs.legacyPackages.${system};
        in
        {
          # VM launcher with eval cache disabled to ensure fresh builds during development
          vogix-vm = {
            type = "app";
            program = "${pkgs.writeShellScript "vogix-vm" ''
              echo "Building and launching VM with eval cache disabled..."
              nix build .#nixosConfigurations.vogix-test-vm.config.system.build.vm \
                --option eval-cache false \
                --no-link \
                --print-out-paths | while read vm_path; do
                "$vm_path/bin/run-vogix-test-vm"
              done
            ''}";
          };

          # Development helper that disables eval cache to avoid stale results
          # when modifying application modules during active development
          dev-check = {
            type = "app";
            program = "${pkgs.writeShellScript "dev-check" ''
              echo "Running flake checks with eval cache disabled (for development)..."
              nix flake check --option eval-cache false "$@"
            ''}";
          };
        }
      );
    };
}

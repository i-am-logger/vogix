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
        vogix16Themes = inputs.vogix16-themes;
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
          default = vogix;
        }
      );

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
              # Negative case: a horizontal-only widget on a vertical bar
              # must trip the assertion (checked here at eval, since plain
              # home-manager only surfaces assertions at activation build).
              badConf = home-manager.lib.homeManagerConfiguration {
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
                      desktop = {
                        enable = true;
                        bars.left.layout.start = [ "window" ];
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
              # home-manager gates all of `config` behind its assertion check
              # (any access throws the "Failed assertions" error), so the
              # negative case is observed as eval failure — the good config
              # above evaluating cleanly is what rules out unrelated breakage.
              verticalRejected = !(builtins.tryEval badConf.config.home.username).success;
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
              customGuarded =
                customRejected { bars.top.layout.end = [ "custom/nope" ]; }
                && customRejected { custom."a/b".command = "date"; };
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
            assert verticalRejected || throw "a horizontal-only widget on bars.left did not trip the vertical-bar assertion";
            assert customGuarded || throw "an undefined custom/<name> placement or a bad custom cell name did not trip its assertion";
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
              # Tailscale, systemd) that must match the running daemon.
              hostProvided=" sh readlink uname pkill systemctl hyprctl tailscale "
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
          # the 0.3.1 sources); everything else must be clean.
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
                --uncreatable-type disable --unresolved-type disable \
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

          # The shell actually RUNS: a headless cage compositor hosts the real
          # quickshell loading the real QML against fixture contract files,
          # and the bar IPC round-trips (status → toggle → status). The full
          # Hyprland-hosted test comes with the VM desktop suite; this pins
          # "the QML loads and the verbs answer" on every check, no VM needed.
          desktop-smoke =
            let
              qsPkgs = import nixpkgs {
                inherit system;
                overlays = [ self.overlays.default ];
                config.allowUnfreePredicate = pkg: builtins.elem (nixpkgs.lib.getName pkg) unfreePackageNames;
              };
              themeJson = builtins.toJSON {
                schema = 1;
                theme = "smoke";
                variant = "night";
                scheme = "vogix16";
                polarity = "dark";
                backgrounds = [ ];
                palette = { base00 = "#101010"; };
                semantic = {
                  active = "#1c1c1c";
                  background = "#101010";
                  background_selection = "#121212";
                  background_surface = "#111111";
                  danger = "#1b1b1b";
                  foreground_border = "#141414";
                  foreground_bright = "#171717";
                  foreground_comment = "#131313";
                  foreground_heading = "#161616";
                  foreground_text = "#151515";
                  highlight = "#1e1e1e";
                  link = "#1d1d1d";
                  notice = "#1a1a1a";
                  special = "#1f1f1f";
                  success = "#181818";
                  warning = "#191919";
                };
              };
              desktopJson = builtins.toJSON {
                schema = 2;
                font = { family = "monospace"; size = 16; };
                # The real default catalog: every widget the shell ships
                # instantiates here, so a widget that fails to load fails
                # the smoke.
                bars = {
                  top = {
                    enable = true;
                    size = 36;
                    layout = {
                      start = [ "workspaces" "mode" ];
                      center = [ "window" ];
                      end = [ "spectrum-mini" "kbd" "privacy" "dnd" "indicators" "theme" "clock" "custom/smoke" "custom/gauge" ];
                    };
                  };
                  bottom = {
                    enable = true;
                    size = 32;
                    layout = {
                      start = [ "media" "spacer" ];
                      center = [ "stat-cpu" "stat-temp" "stat-mem" "stat-swap" "stat-disk" "stat-mounts" "stat-net" "vu-out" "vu-mic" ];
                      end = [ "tailscale" "weather" "update" "tray" ];
                    };
                  };
                  left = {
                    enable = true;
                    size = 36;
                    layout = { start = [ "workspaces" "mode" ]; center = [ ]; end = [ "menu" "power-glyph" ]; };
                  };
                  right = {
                    enable = true;
                    size = 48;
                    layout = {
                      start = [ "vu-rail" "audio-out-picker" "audio-in-picker" "spectrum-rail" ];
                      center = [ "graph-cpu" "graph-mem" "graph-net" "custom/watched" "custom/counter" "custom/stream" ];
                      end = [ "batteries" "battery" "audio" "mic" "network" "bluetooth" ];
                    };
                  };
                };
                # One mount every host has and one no host has: the mounts
                # cell must come up for the first and simply not exist for
                # the second.
                meters.mounts = [ "/" "/vogix-smoke-absent" ];
                # Custom cells over every trigger but the timer: first show
                # (text, json, a stream), a watched file's creation and
                # change, and the IPC refresh. @RT@ becomes the runtime
                # dir once the file is in place.
                custom = {
                  smoke = { title = "SMK"; command = "echo SMOKE-42"; };
                  gauge = {
                    command = "printf '%s' '{\"text\":\"J-7\",\"state\":\"danger\",\"meter\":0.5}'";
                    output = "json";
                  };
                  stream = { command = "echo S-1; echo S-2"; stream = true; };
                  watched = { command = "cat @RT@/smoke-watch"; watch = [ "@RT@/smoke-watch" ]; };
                  counter = {
                    command = "n=$(cat @RT@/smoke-count 2>/dev/null || echo 0); n=$((n + 1)); echo $n > @RT@/smoke-count; echo RUN-$n";
                  };
                };
                surfaces.bar = {
                  background = { slot = "background"; alpha = 0.92; };
                  foreground = { slot = "foreground_text"; alpha = 1.0; };
                  muted = { slot = "foreground_comment"; alpha = 1.0; };
                  accent = { slot = "active"; alpha = 1.0; };
                  border = { slot = "foreground_border"; alpha = 1.0; };
                  urgent = { slot = "danger"; alpha = 1.0; };
                };
              };
              # `hyprctl -j devices` as Hyprland answers it (HyprCtl.cpp's
              # keyboards shape); every other call fails as it does with no
              # compositor. The keyboard the LANG cell follows is neither first
              # nor main, and its active layout's name ("German") does not
              # abbreviate to its code, so only the right device plus the
              # reported index yield "active:de".
              hyprctlFixture =
                let
                  devices = pkgs.writeText "hyprctl-devices.json" (builtins.toJSON {
                    mice = [ ];
                    keyboards = [
                      {
                        address = "0x1";
                        name = "at-translated-set-2-keyboard";
                        rules = "";
                        model = "";
                        layout = "us";
                        variant = "";
                        options = "";
                        active_layout_index = 0;
                        active_keymap = "English (US)";
                        capsLock = false;
                        numLock = false;
                        main = true;
                      }
                      {
                        address = "0x2";
                        name = "vogix-input";
                        rules = "";
                        model = "";
                        layout = "de,us";
                        variant = "";
                        options = "grp:alt_caps_toggle";
                        active_layout_index = 0;
                        active_keymap = "German";
                        capsLock = false;
                        numLock = false;
                        main = false;
                      }
                    ];
                    tablets = [ ];
                    touch = [ ];
                    switches = [ ];
                  });
                in
                pkgs.writeShellScriptBin "hyprctl" ''
                  if [ "$*" = "-j devices" ]; then
                    cat ${devices}
                  else
                    exit 1
                  fi
                '';
              # A schema-1 desktop.json (the single-bar shape, no `bars`).
              # The shell reads schema 2 only: it must stay up, render no
              # bar, and say why in its log.
              schema1Json = builtins.toJSON {
                schema = 1;
                font = { family = "monospace"; size = 13; };
                bar = {
                  enable = true;
                  position = "top";
                  height = 32;
                  layout = { left = [ "clock" ]; center = [ ]; right = [ ]; };
                };
                surfaces.bar = {
                  background = { slot = "background"; alpha = 0.92; };
                  foreground = { slot = "foreground_text"; alpha = 1.0; };
                  muted = { slot = "foreground_comment"; alpha = 1.0; };
                  accent = { slot = "active"; alpha = 1.0; };
                  border = { slot = "foreground_border"; alpha = 1.0; };
                  urgent = { slot = "danger"; alpha = 1.0; };
                };
              };
            in
            pkgs.runCommand "vogix-desktop-smoke"
              {
                nativeBuildInputs = [ pkgs.cage qsPkgs.quickshell hyprctlFixture ];
                qml = qsPkgs.vogix-desktop-qml;
                geometryProbe = ./nix/checks/desktop-geometry-probe.qml;
                inherit themeJson desktopJson schema1Json;
                passAsFile = [ "themeJson" "desktopJson" "schema1Json" ];
              } ''
              export HOME=$TMPDIR/home
              export XDG_CONFIG_HOME=$HOME/.config
              export XDG_STATE_HOME=$HOME/.local/state
              export XDG_RUNTIME_DIR=$TMPDIR/rt
              mkdir -p $XDG_CONFIG_HOME/vogix-desktop $XDG_STATE_HOME/vogix/desktop $XDG_RUNTIME_DIR
              chmod 700 $XDG_RUNTIME_DIR
              cp $themeJsonPath $XDG_CONFIG_HOME/vogix-desktop/theme.json
              cp $desktopJsonPath $XDG_STATE_HOME/vogix/desktop.json
              sed -i "s|@RT@|$XDG_RUNTIME_DIR|g" $XDG_STATE_HOME/vogix/desktop.json

              # The geometry probe replaces shell.qml in a copy of the tree,
              # so `qs.` resolves to the shipped modules around it.
              cp -r $qml $TMPDIR/probe
              chmod -R u+w $TMPDIR/probe
              cp $geometryProbe $TMPDIR/probe/shell.qml

              cat > inner.sh <<INNER
              #!${pkgs.runtimeShell}
              export QS_NO_RELOAD_POPUP=1 QS_DISABLE_FILE_WATCHER=1
              # No GPU and no GL in the build sandbox: render the scene with
              # Qt's software adaptation so the smoke also proves the bar
              # actually paints, not just that the engine loads.
              export QT_QUICK_BACKEND=software
              qs -p $qml > $TMPDIR/qs.log 2>&1 &
              QSPID=\$!
              sleep 5
              kill -0 \$QSPID 2>/dev/null && echo ALIVE >> $TMPDIR/result
              qs -p $qml ipc call bar status >> $TMPDIR/result 2>&1
              qs -p $qml ipc call bar hide left >> $TMPDIR/result 2>&1
              qs -p $qml ipc call bar unhide all >> $TMPDIR/result 2>&1
              qs -p $qml ipc call bar toggle all >> $TMPDIR/result 2>&1
              qs -p $qml ipc call bar toggle all >> $TMPDIR/result 2>&1
              qs -p $qml ipc call theme reload >> $TMPDIR/result 2>&1
              qs -p $qml ipc call launcher status >> $TMPDIR/result
              qs -p $qml ipc call power toggle >> $TMPDIR/result
              qs -p $qml ipc call power status >> $TMPDIR/result
              qs -p $qml ipc call power close >> $TMPDIR/result 2>&1
              qs -p $qml ipc call panel toggle calendar >> $TMPDIR/result
              qs -p $qml ipc call panel status >> $TMPDIR/result
              qs -p $qml ipc call panel toggle audio-out >> $TMPDIR/result
              qs -p $qml ipc call panel toggle audio-in >> $TMPDIR/result
              qs -p $qml ipc call panel close >> $TMPDIR/result 2>&1
              qs -p $qml ipc call stayawake toggle >> $TMPDIR/result
              qs -p $qml ipc call stayawake status >> $TMPDIR/result
              qs -p $qml ipc call nightlight status >> $TMPDIR/result
              qs -p $qml ipc call gallery open >> $TMPDIR/result
              qs -p $qml ipc call gallery status >> $TMPDIR/result
              qs -p $qml ipc call gallery close >> $TMPDIR/result 2>&1
              qs -p $qml ipc call reminders list >> $TMPDIR/result
              for cell in smoke gauge stream watched undefined; do
                qs -p $qml ipc call custom status \$cell | sed "s/^/custom-\$cell /" >> $TMPDIR/result
              done
              echo W-1 > $XDG_RUNTIME_DIR/smoke-watch
              sleep 1
              qs -p $qml ipc call custom status watched | sed 's/^/custom-watched-created /' >> $TMPDIR/result
              echo W-2 > $XDG_RUNTIME_DIR/smoke-watch
              sleep 1
              qs -p $qml ipc call custom status watched | sed 's/^/custom-watched-changed /' >> $TMPDIR/result
              qs -p $qml ipc call custom refresh counter | sed 's/^/custom-counter-refresh /' >> $TMPDIR/result
              sleep 1
              qs -p $qml ipc call custom status counter | sed 's/^/custom-counter /' >> $TMPDIR/result
              qs -p $qml ipc call keyboard status >> $TMPDIR/result
              # The input engine's lock document, handled as the engine does:
              # written tmp + rename, rewritten the same way, removed on stop.
              # The LANG cell must follow every step through its file watch.
              put_locks() {
                printf '%s' "\$1" > $XDG_STATE_HOME/vogix/input-locks.json.tmp
                mv $XDG_STATE_HOME/vogix/input-locks.json.tmp $XDG_STATE_HOME/vogix/input-locks.json
              }
              caps_until() {
                for _ in \$(seq 50); do
                  s=\$(qs -p $qml ipc call keyboard status)
                  case "\$s" in *" caps:\$1") break ;; esac
                  sleep 0.1
                done
                echo "\$2 \$s" >> $TMPDIR/result
              }
              put_locks '{"capsLock":true,"numLock":false,"scrollLock":null}'
              caps_until on locks-on
              put_locks '{"capsLock":false,"numLock":false,"scrollLock":null}'
              caps_until off locks-off
              put_locks '{"capsLock":null,"numLock":null,"scrollLock":null}'
              caps_until unknown locks-null
              put_locks '{"capsLock":true,"numLock":false,"scrollLock":null}'
              caps_until on locks-back
              rm $XDG_STATE_HOME/vogix/input-locks.json
              caps_until unknown locks-gone
              kill \$QSPID 2>/dev/null || true
              sleep 1

              # Geometry run: the probe exits on its own with its verdict;
              # the timeout only bounds a probe that never completes.
              timeout 30 qs -p $TMPDIR/probe > $TMPDIR/qs-probe.log 2>&1
              echo "PROBE-EXIT \$?" >> $TMPDIR/result

              # Rejection run: a schema-1 desktop.json is refused, loudly,
              # without taking the shell down.
              cp $schema1JsonPath $XDG_STATE_HOME/vogix/desktop.json
              qs -p $qml > $TMPDIR/qs-schema1.log 2>&1 &
              QSPID=\$!
              sleep 3
              kill -0 \$QSPID 2>/dev/null && echo SCHEMA1-ALIVE >> $TMPDIR/result
              qs -p $qml ipc call bar status | sed 's/^/schema1 /' >> $TMPDIR/result
              kill \$QSPID 2>/dev/null || true
              INNER
              chmod +x inner.sh
              WLR_BACKENDS=headless WLR_LIBINPUT_NO_DEVICES=1 WLR_RENDERER=pixman \
                cage -- ./inner.sh || true

              echo "── result:"; cat $TMPDIR/result || true
              echo "── log tail:"; tail -5 $TMPDIR/qs.log || true
              echo "── geometry:"; grep GEOMETRY $TMPDIR/qs-probe.log || true
              grep -q '^ALIVE$' $TMPDIR/result
              # A canvas instrument must never lay out collapsed: the probe's
              # verdict, and the oscilloscope's own measured size.
              grep -q '^PROBE-EXIT 0$' $TMPDIR/result
              grep -q 'GEOMETRY oscilloscope [1-9][0-9.]*x[1-9][0-9.]*$' $TMPDIR/qs-probe.log
              test "$(grep -c '^top:shown bottom:shown left:shown right:shown$' $TMPDIR/result)" -ge 3
              grep -q 'left:hidden' $TMPDIR/result
              grep -q 'top:hidden bottom:hidden left:hidden right:hidden' $TMPDIR/result
              grep -q '^SCHEMA1-ALIVE$' $TMPDIR/result
              grep -q '^schema1 top:off bottom:off left:off right:off$' $TMPDIR/result
              grep -q 'desktop.json schema 1 is not supported' $TMPDIR/qs-schema1.log
              grep -q '^closed$' $TMPDIR/result
              grep -q '^open$' $TMPDIR/result
              grep -q '^calendar$' $TMPDIR/result
              grep -q '^audio-out$' $TMPDIR/result
              grep -q '^audio-in$' $TMPDIR/result
              grep -q '^on$' $TMPDIR/result
              grep -q '^off$' $TMPDIR/result
              grep -q '^no reminders$' $TMPDIR/result
              grep -q '^custom-smoke SMOKE-42$' $TMPDIR/result
              grep -q '^custom-gauge J-7$' $TMPDIR/result
              grep -q '^custom-stream S-2$' $TMPDIR/result
              grep -q '^custom-watched failed: exited 1$' $TMPDIR/result
              grep -q '^custom-watched-created W-1$' $TMPDIR/result
              grep -q '^custom-watched-changed W-2$' $TMPDIR/result
              grep -q '^custom-counter-refresh refreshing$' $TMPDIR/result
              grep -q '^custom-counter RUN-2$' $TMPDIR/result
              grep -q '^custom-undefined unknown custom cell: undefined$' $TMPDIR/result
              grep -q '^device:vogix-input layouts:de,us active:de caps:unknown$' $TMPDIR/result
              grep -q '^locks-on device:vogix-input layouts:de,us active:de caps:on$' $TMPDIR/result
              grep -q '^locks-off .* caps:off$' $TMPDIR/result
              grep -q '^locks-null .* caps:unknown$' $TMPDIR/result
              grep -q '^locks-back .* caps:on$' $TMPDIR/result
              grep -q '^locks-gone .* caps:unknown$' $TMPDIR/result
              ! grep -iq 'is not a type\|module .* is not installed\|Failed to load configuration' $TMPDIR/qs.log
              touch $out
            '';

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
                          path = ./README.md;
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

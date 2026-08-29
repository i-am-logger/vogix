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
            config.allowUnfreePredicate = pkg: builtins.elem (nixpkgs.lib.getName pkg) [ "vogix" ];
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
        in
        {
          inherit vogix;
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
          inherit (self.packages.${prev.stdenv.hostPlatform.system}) vogix;
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
            nixpkgs.config.allowUnfreePredicate = pkg: builtins.elem (nixpkgs.lib.getName pkg) [ "vogix" ];

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
            config.allowUnfreePredicate = pkg: builtins.elem (nixpkgs.lib.getName pkg) [ "vogix" ];
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
              };
            in
            pkgs.runCommand "vogix-nix-unit"
              (builtins.mapAttrs (_: toString) suites)
              ''
                echo "appearance: $appearance  behavior: $behavior  hyprland: $hyprland"
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
          # exitAfter, the Esc safety-net, repeat, and the single-instance guard.
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
      #       config.allowUnfreePredicate = pkg: builtins.elem (nixpkgs.lib.getName pkg) [ "vogix" ];
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

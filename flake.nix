{
  description = "Vogix - Runtime theme management for NixOS";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    home-manager = {
      url = "github:nix-community/home-manager";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    devenv = {
      url = "github:cachix/devenv";
      inputs = {
        nixpkgs.follows = "nixpkgs";
        cachix.follows = "crate2nix/cachix";
        flake-compat.follows = "crate2nix/flake-compat";
        flake-parts.follows = "crate2nix/flake-parts";
        git-hooks.inputs.gitignore.follows = "crate2nix/pre-commit-hooks/gitignore";
        crate2nix.follows = "crate2nix";
        rust-overlay.follows = "rust-overlay";
      };
    };
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    crate2nix = {
      url = "github:i-am-logger/crate2nix/fix/remove-crate2nix-stable-input";
      inputs = {
        nixpkgs.follows = "nixpkgs";
        cachix.inputs.nixpkgs.follows = "nixpkgs";
      };
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
  };

  nixConfig = {
    extra-trusted-public-keys = "devenv.cachix.org-1:w1cLUi8dv3hnoSPGAuibQv+f9TZLr6cv/Hm9XgU50cw=";
    extra-substituters = "https://devenv.cachix.org";
  };

  outputs =
    { self
    , nixpkgs
    , home-manager
    , devenv
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
      # NixOS module (console colors + security wrappers only)
      nixosModules.default = import ./nix/modules/nixos.nix {
        vogix16Themes = inputs.vogix16-themes;
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
          # Use mkConfig but only access outputs, not shell
          devenvOutputs =
            (devenv.lib.mkConfig {
              inherit inputs pkgs;
              modules = [ ./devenv.nix ];
            }).outputs;
        in
        devenvOutputs
        // {
          default = devenvOutputs.vogix;
        }
      );

      # Overlay to make vogix available in pkgs
      overlays.default = _final: prev: {
        inherit (self.packages.${prev.system}) vogix;
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

          # Runtime size inspection
          runtime-size = import ./nix/vm/tests/runtime-size.nix testArgs;

          # Rapid switching tests
          stress = import ./nix/vm/tests/stress.nix testArgs;

          # Template architecture tests
          templates = import ./nix/vm/tests/templates.nix testArgs;
        }
      );

      # Development shells - using devenv
      # Note: Use 'devenv shell' for development instead of 'nix develop'
      # devShells commented out due to Windows dependencies issue in crate2nix evaluation
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

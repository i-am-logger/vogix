{ pkgs
, config
, lib
, ...
}:
let
  cargoToml = builtins.fromTOML (builtins.readFile ./Cargo.toml);
  packageName = cargoToml.package.name;
  packageVersion = cargoToml.package.version;
  packageDescription = cargoToml.package.description or "";
in
{
  # Set root explicitly for flake compatibility
  devenv.root = lib.mkDefault (builtins.toString ./.);

  dotenv.enable = true;
  imports = [
    ./nix/rust.nix
  ];

  # Additional packages for development
  packages = [
    pkgs.git
    pkgs.dbus
    pkgs.pkg-config
  ];

  # Development scripts
  scripts.dev-test.exec = ''
    echo "🧪 Running tests..."
    cargo test --all-features
  '';

  scripts.dev-run.exec = ''
    echo "🚀 Running vogix..."
    cargo run --release
  '';

  scripts.dev-build.exec = ''
    echo "🔨 Building vogix..."
    cargo build --release
  '';

  # Nix development scripts (disable eval cache to avoid stale results)
  scripts.nix-build-dev.exec = ''
    echo "🏗️  Building VM with eval cache disabled (for active development)..."
    nix build .#nixosConfigurations.vogix-test-vm.config.system.build.vm \
      --option eval-cache false
  '';

  scripts.nix-check-dev.exec = ''
    echo "✅ Checking flake with eval cache disabled (for active development)..."
    nix flake check --option eval-cache false
  '';

  # Environment variables
  env = {
    PROJECT_NAME = "vogix";
    CARGO_TARGET_DIR = "./target";
  };

  # Development shell setup
  enterShell = ''
    clear
    ${pkgs.figlet}/bin/figlet "${packageName}"
    echo
    {
      ${pkgs.lib.optionalString (packageDescription != "") ''echo "• ${packageDescription}"''}
      echo -e "• \033[1mv${packageVersion}\033[0m"
      echo -e " \033[0;32m✓\033[0m Development environment ready"
    } | ${pkgs.boxes}/bin/boxes -d stone -a l -i none
    echo
    echo "Available scripts:"
    echo "  Rust Development:"
    echo "    • dev-test      - Run tests"
    echo "    • dev-run       - Run the application"
    echo "    • dev-build     - Build the application"
    echo ""
    echo "  Nix Development (eval cache disabled):"
    echo "    • nix-build-dev - Build VM without eval cache"
    echo "    • nix-check-dev - Check flake without eval cache"
    echo ""
  '';

  # https://devenv.sh/integrations/treefmt/
  treefmt = {
    enable = true;
    config = {
      # Global exclusions
      settings.global.excludes = [
        # Devenv generated files
        ".devenv.flake.nix"
        ".devenv/"
      ];

      programs = {
        # Nix
        nixpkgs-fmt.enable = true;
        deadnix = {
          enable = true;
          no-underscore = true; # Don't warn about bindings starting with _
        };
        statix.enable = true;

        # Rust — use devenv toolchain (supports edition 2024)
        rustfmt = {
          enable = true;
          package = config.languages.rust.toolchainPackage;
        };

        # Shell
        shellcheck.enable = true;
        shfmt.enable = true;
      };
    };
  };

  # https://devenv.sh/git-hooks/
  git-hooks.settings.rust.cargoManifestPath = "./Cargo.toml";

  # Use the same Rust toolchain for git-hooks as for development
  # This ensures clippy/rustfmt versions match the devenv shell
  # mkForce is needed to override the default from languages.rust module
  git-hooks.tools = {
    cargo = lib.mkForce config.languages.rust.toolchainPackage;
    clippy = lib.mkForce config.languages.rust.toolchainPackage;
    rustfmt = lib.mkForce config.languages.rust.toolchainPackage;
  };

  git-hooks.hooks = {
    treefmt.enable = true;
    clippy.enable = true;
  };

  # NOTE: the Nix PACKAGE build of vogix is no longer here. It moved to nixpkgs'
  # standard `buildRustPackage` (nix/packages/vogix.nix, wired in flake.nix),
  # because praxis 0.24.0 adopted `version.workspace = true` and crate2nix's
  # per-crate vendor step can't resolve a workspace-inherited version (no
  # workspace root), whereas cargo's vendoring fetches the whole praxis repo and
  # resolves it. devenv stays for the dev shell + treefmt + git-hooks + tasks.

  # https://devenv.sh/tasks/
  tasks = {
    "test:fmt" = {
      exec = "treefmt --fail-on-change";
    };

    "test:clippy" = {
      exec = "cargo clippy -p vogix --quiet -- -D warnings";
    };

    "test:check" = {
      exec = "cargo check -p vogix --quiet";
    };

    "test:unit" = {
      exec = "cargo test -p vogix --quiet";
    };
  };

  # https://devenv.sh/tests/
  # Use mkForce to override devenv's default enterTest which exports bash functions
  # that cause issues with treefmt subprocesses (black, etc.)
  enterTest = lib.mkForce "devenv tasks run test:fmt test:clippy test:check test:unit";

  # See full reference at https://devenv.sh/reference/options/
}

{ pkgs
, config
, lib
, inputs
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

        # Python
        black.enable = true;

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

  # https://devenv.sh/outputs/
  outputs =
    let
      # Vendor praxis into the source tree for Nix builds
      # Patches Cargo.toml path from ../praxis/crates/praxis to .nix-deps/praxis
      nixSrc = pkgs.runCommand "vogix-nix-src" { } ''
        cp -r ${./.} $out
        chmod -R u+w $out
        mkdir -p $out/.nix-deps/praxis
        cp -r ${inputs.praxis-src}/crates/praxis/. $out/.nix-deps/praxis/
        substituteInPlace $out/Cargo.toml \
          --replace-fail '../praxis/crates/praxis' '.nix-deps/praxis'
      '';
    in
    {
      vogix = config.languages.rust.import nixSrc {
        # Override to skip Windows-specific dependencies
        crateOverrides = pkgs.defaultCrateOverrides // {
          # Skip all Windows-specific crates
          windows-sys = _attrs: null;
          windows-core = _attrs: null;
          windows-targets = _attrs: null;
          windows_x86_64_gnu = _attrs: null;
          windows_x86_64_msvc = _attrs: null;
          windows_i686_gnu = _attrs: null;
          windows_i686_msvc = _attrs: null;
          windows_aarch64_msvc = _attrs: null;
          windows_aarch64_gnullvm = _attrs: null;
          anstyle-wincon = _attrs: null;
        };
      };
    };

  # https://devenv.sh/tasks/
  tasks = {
    "test:fmt" = {
      exec = "treefmt --fail-on-change";
    };

    "test:clippy" = {
      exec = "cargo clippy --quiet -- -D warnings";
    };

    "test:check" = {
      exec = "cargo check --quiet";
    };

    "test:unit" = {
      exec = "cargo test --quiet";
    };
  };

  # https://devenv.sh/tests/
  # Use mkForce to override devenv's default enterTest which exports bash functions
  # that cause issues with treefmt subprocesses (black, etc.)
  enterTest = lib.mkForce "devenv tasks run test:fmt test:clippy test:check test:unit";

  # See full reference at https://devenv.sh/reference/options/
}

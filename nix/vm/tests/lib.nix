# Shared test utilities and VM configuration
#
# This module provides common setup for all integration tests:
# - VM configuration (nodes.machine)
# - Theme data (allThemes, themesJSON)
# - Common Python test helpers
#
{ pkgs
, home-manager
, self
, vogix16Themes
,
}:

let
  inherit (pkgs) lib;

  # Import the test-vm configuration
  testVMConfig = import ../test-vm.nix;

  # Import vogix16 themes using the importer module
  vogix16Import = import ../../modules/vogix16-import.nix {
    inherit lib vogix16Themes;
  };

  # Convert to test-compatible format (theme -> {variant -> colors})
  allThemes = builtins.mapAttrs
    (
      _themeName: theme: builtins.mapAttrs (_variantName: variantData: variantData.colors) theme.variants
    )
    vogix16Import.themes;

  themesJSON = builtins.toJSON allThemes;

  # Common VM node configuration
  machineConfig = {
    imports = [
      testVMConfig
      self.nixosModules.default
      home-manager.nixosModules.home-manager
    ];

    nixpkgs.overlays = [
      (_final: _prev: {
        inherit (self.packages.x86_64-linux) vogix;
      })
    ];

    home-manager = {
      useGlobalPkgs = true;
      useUserPackages = true;
      users.vogix = import ../home.nix;
      sharedModules = [ self.homeManagerModules.default ];
    };
  };

  # Common Python preamble for all test scripts
  testPreamble = ''
    import time
    import json
    import shlex
    import re

    # Ensure imports are used (prevents lint warnings in tests that don't need them)
    _ = (shlex.quote, re.search)

    # Load all theme colors from Nix
    all_themes = json.loads(r"""${themesJSON}""")
    yoga_colors = all_themes['yoga']

    # Start the machine
    machine.start()
    machine.wait_for_unit("multi-user.target")

    # Wait for user session to be ready
    time.sleep(2)

    # Path definitions for the architecture:
    # - User config: ~/.local/state/vogix/config.toml (home-manager generated)
    # - User themes: ~/.local/share/vogix/themes/{theme}-{variant}/ (home-manager)
    # - User state: ~/.local/state/vogix/ (CLI managed)
    # - Current theme symlink: ~/.local/state/vogix/current-theme
    vogix_state = "/home/vogix/.local/state/vogix"
    vogix_config = vogix_state  # Config is now in state dir, not /etc
    vogix_themes = "/home/vogix/.local/share/vogix/themes"
    current_theme = f"{vogix_state}/current-theme"
  '';

  # Helper to create a test with common setup
  mkTest =
    name: testScript:
    pkgs.testers.nixosTest {
      name = "vogix-${name}";
      nodes.machine = _: machineConfig;
      testScript = testPreamble + testScript;
    };

in
{
  inherit
    allThemes
    themesJSON
    machineConfig
    testPreamble
    mkTest
    ;
}

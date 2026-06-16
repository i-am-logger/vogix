{ config, lib, ... }:

let
  inherit (lib)
    mkIf
    mkDefault
    ;

  cfg = config.vogix.hardware.dram-rgb;
  # Chipset family auto-detected by motherboard.nix from the NixOS CPU config
  # ("amd"/"intel"/null). The RGB lives on the RAM modules, but it's reached over
  # the motherboard chipset's SMBus, so we still need the matching driver.
  chipset = config.vogix.hardware.motherboard;
in
{
  config = mkIf cfg.enable {
    # DDR5 RGB is on the memory modules themselves (ENE controllers) -- it's
    # independent of the motherboard and of any keyboard, and travels with the
    # RAM if you swap sticks. It's reachable over the chipset SMBus (i2c), so
    # this module -- not the keyboard -- owns the i2c/OpenRGB stack.
    vogix.openrgb.enable = mkDefault true;

    # nixpkgs' services.hardware.openrgb always loads i2c-dev and, from the
    # `motherboard` (chipset) type, the matching SMBus driver (i2c-piix4 for AMD,
    # i2c-i801 for Intel). We pass that type through and add
    # acpi_enforce_resources=lax, which nixpkgs does NOT set but AMD SMBus access
    # requires -- without it OpenRGB logs "Failed to read i2c device PCI device ID"
    # and DRAM RGB stays unavailable. i2c-dev is listed explicitly so the
    # dependency is visible at this module.
    services.hardware.openrgb.motherboard = mkIf (chipset != null) chipset;
    boot.kernelModules = [ "i2c-dev" ];
    boot.kernelParams = [ "acpi_enforce_resources=lax" ];

    # Theme apply: paint all DRAM modules from the vogix palette on theme change.
    vogix.hardware.themeApply.dram-rgb =
      "openrgb -d 'ENE DRAM' -m static -c {{${cfg.colorSlot}}}";
  };
}

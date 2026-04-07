{ config, lib, ... }:

let
  inherit (lib)
    mkOption
    types
    ;

  # Auto-detect motherboard type from standard NixOS CPU config
  detected =
    if config.hardware.cpu.amd.updateMicrocode or false then "amd"
    else if config.hardware.cpu.intel.updateMicrocode or false then "intel"
    else null;
in
{
  options.vogix.hardware.motherboard = mkOption {
    type = types.nullOr (types.enum [ "amd" "intel" ]);
    default = detected;
    description = "Motherboard type. Auto-detected from NixOS CPU config.";
  };
}

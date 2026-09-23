# The home-manager users with programs.vogix.enable, sorted by name; empty
# when the home-manager NixOS module is not imported.
{ config, options, lib }:

if options ? home-manager then
  lib.attrNames
    (lib.filterAttrs (_name: userCfg: userCfg.programs.vogix.enable or false)
      (config.home-manager.users or { }))
else
  [ ]

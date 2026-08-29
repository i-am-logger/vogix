# Desktop shell options for Vogix
#
# Defines programs.vogix.desktop.* — the vogix desktop shell (bar,
# notifications, lock, launcher, …; quickshell-rendered in v1, the
# vogix-desktop Rust program in v2; the contract both consume is
# `theme.json` + `desktop.json`).
#
# Bare fragment in the behavior-options shape, merged into
# `options.programs.vogix` by home-manager/options.nix. This starts minimal —
# the per-surface trees (bar, notifications, lock, osd, idle, …) grow here
# increment by increment as the plan's steps land.
{ lib }:

{
  desktop = lib.mkOption {
    description = "The vogix desktop shell.";
    default = { };
    type = lib.types.submodule {
      options = {
        enable = lib.mkEnableOption "the vogix desktop shell (bar, notifications, lock, launcher)";
      };
    };
  };
}

# Desktop shell options for Vogix
#
# Defines programs.vogix.desktop.* — the vogix desktop shell (bar,
# notifications, lock, launcher, …; quickshell-rendered in v1, the
# vogix-desktop Rust program in v2; the contract both consume is
# `theme.json` + `desktop.json`).
#
# Bare fragment in the behavior-options shape, merged into
# `options.programs.vogix` by home-manager/options.nix. The per-surface
# trees grow here increment by increment as the plan's steps land.
{ lib }:

let
  inherit (lib) mkOption mkEnableOption types;
  defaults = import ./defaults.nix { };
  v16 = import ../lib/vogix16.nix { inherit lib; };

  # A surface color token: a semantic SLOT (one of the 16 praxis keys) plus
  # an alpha. `background = "background"` coerces to `{ slot; alpha = 1.0; }`
  # — the praxis projection is SlotMapping { slot, target_key, Rgba { alpha } }.
  tokenType = types.coercedTo types.str (slot: { inherit slot; }) (types.submodule {
    options = {
      slot = mkOption {
        type = types.enum v16.semanticKeys;
        description = "Semantic slot this token resolves through the current theme.";
      };
      alpha = mkOption {
        type = types.numbers.between 0.0 1.0;
        default = 1.0;
        description = "Opacity applied to the resolved color.";
      };
    };
  });

  widgetNames = types.listOf types.str;

in
{
  desktop = mkOption {
    description = "The vogix desktop shell.";
    default = { };
    type = types.submodule {
      options = {
        enable = mkEnableOption "the vogix desktop shell (bar, notifications, lock, launcher)";

        font = {
          family = mkOption {
            type = types.str;
            default = defaults.font.family;
            description = "Shell UI font family.";
          };
          size = mkOption {
            type = types.ints.positive;
            default = defaults.font.size;
            description = "Base font size (logical px); shell type scale derives from it.";
          };
        };

        bar = {
          enable = mkOption {
            type = types.bool;
            default = defaults.bar.enable;
            description = "Render the bar (one per monitor).";
          };
          position = mkOption {
            type = types.enum [ "top" "bottom" ];
            default = defaults.bar.position;
            description = "Screen edge the bar anchors to.";
          };
          height = mkOption {
            type = types.ints.positive;
            default = defaults.bar.height;
            description = "Bar height (logical px).";
          };
          layout = {
            left = mkOption {
              type = widgetNames;
              default = defaults.bar.layout.left;
              description = "Widgets in the left section, in order.";
            };
            center = mkOption {
              type = widgetNames;
              default = defaults.bar.layout.center;
              description = "Widgets in the center section, in order.";
            };
            right = mkOption {
              type = widgetNames;
              default = defaults.bar.layout.right;
              description = "Widgets in the right section, in order.";
            };
          };
        };

        surfaces = mkOption {
          type = types.attrsOf (types.attrsOf tokenType);
          default = { };
          description = ''
            Per-surface color tokens ({ slot, alpha }, or a bare slot name).
            Surfaces are free-form names (bar, popup, notification, lock, …);
            `vogix desktop check` projects them onto praxis surface functors
            and rejects unknown slots. Defaults for the shipped surfaces merge
            underneath (defaults.nix).
          '';
        };
      };
    };
  };
}

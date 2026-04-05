# Behavior options for Vogix
#
# Defines programs.vogix.behavior.* options
# Two sub-domains:
#   - keybindings: input config (modKey, mouse, layers)
#   - modes: modal system (desktop, arrange, theme)
{ lib }:

let
  inherit (lib)
    mkOption
    mkEnableOption
    types
    ;

  keyComboType = types.str;

  # Mode binding type
  modeBindingType = types.submodule {
    options = {
      enter = mkOption {
        type = types.nullOr keyComboType;
        default = null;
        description = "Key combo to enter this mode (null for the default/app mode)";
      };

      exit = mkOption {
        type = keyComboType;
        default = "escape";
        description = "Key to exit this mode";
      };

      bindings = mkOption {
        type = types.attrsOf (types.submodule {
          options = {
            key = mkOption {
              type = keyComboType;
              description = "Key combination for this action";
            };
            action = mkOption {
              type = types.str;
              description = "Action to perform";
            };
            description = mkOption {
              type = types.str;
              default = "";
              description = "Human-readable description (used for help/discovery)";
            };
            repeat = mkOption {
              type = types.bool;
              default = false;
              description = "Whether this binding repeats when held";
            };
          };
        });
        default = { };
        description = "Named bindings in this mode";
      };
    };
  };

  # Kanata layer type
  layerType = types.submodule {
    options = {
      toggle = mkOption {
        type = types.nullOr types.str;
        default = null;
        description = "Key that toggles this layer on/off";
      };
      hold = mkOption {
        type = types.nullOr types.str;
        default = null;
        description = "Key that activates this layer while held";
      };
      tapAction = mkOption {
        type = types.nullOr types.str;
        default = null;
        description = "What the toggle/hold key does on tap";
      };
      tapHoldMs = mkOption {
        type = types.int;
        default = 200;
        description = "Tap-hold threshold in milliseconds";
      };
      bindings = mkOption {
        type = types.attrsOf types.str;
        default = { };
        description = "Key remappings in this layer (source = target)";
      };
    };
  };

  # Mouse binding type
  mouseBindingType = types.submodule {
    options = {
      button = mkOption {
        type = types.str;
        description = "Mouse button (e.g., 'mouse:272')";
      };
      action = mkOption {
        type = types.str;
        description = "Action to perform";
      };
      description = mkOption {
        type = types.str;
        default = "";
        description = "Human-readable description";
      };
    };
  };

in
{
  behavior = mkOption {
    description = "Behavior configuration (how things act)";
    default = { };
    type = types.submodule {
      options = {
        # ── Keybindings (input config) ──
        keybindings = mkOption {
          description = "Key configuration — modifier, mouse, input layers";
          default = { };
          type = types.submodule {
            options = {
              modKey = mkOption {
                type = types.enum [ "super" "alt" "ctrl" "meta" ];
                default = "super";
                description = "Primary modifier key (acts as macOS Command — implies Super→Ctrl remap)";
              };

              mouse = mkOption {
                type = types.attrsOf mouseBindingType;
                default = { };
                description = "Mouse button bindings (always available, not mode-specific)";
              };

              layers = mkOption {
                type = types.attrsOf layerType;
                default = { };
                description = "System-wide key layers via kanata (evdev level)";
              };
            };
          };
        };

        # ── Modes (modal system) ──
        modes = mkOption {
          description = "Modal interaction system — contextual modes with single-key actions";
          default = { };
          type = types.submodule {
            options = {
              enable = mkEnableOption "vogix modal interaction system";

              app = mkOption {
                type = modeBindingType;
                default = { };
                description = "App mode (default) — keys pass to apps, global bindings";
              };

              desktop = mkOption {
                type = modeBindingType;
                default = { };
                description = "Desktop mode — WM commands with single keys";
              };

              arrange = mkOption {
                type = modeBindingType;
                default = { };
                description = "Arrange mode — move + resize windows";
              };

              theme = mkOption {
                type = modeBindingType;
                default = { };
                description = "Theme mode — vogix appearance switching";
              };

              modeColors = mkOption {
                type = types.attrsOf (types.submodule {
                  options = {
                    active = mkOption { type = types.str; description = "Active border color"; };
                    inactive = mkOption { type = types.str; description = "Inactive border color"; };
                  };
                });
                internal = true;
                default = { };
                description = "Per-mode border colors derived from vogix theme";
              };
            };
          };
        };

        # ── Internal generated outputs ──
        generatedHyprland = mkOption {
          type = types.attrsOf types.anything;
          internal = true;
          default = { };
          description = "Generated Hyprland config";
        };

        generatedKanata = mkOption {
          type = types.nullOr types.str;
          internal = true;
          default = null;
          description = "Generated kanata config";
        };
      };
    };
  };
}

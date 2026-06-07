# Behavior options for Vogix
#
# Defines programs.vogix.behavior.* options
# Two sub-domains:
#   - keybindings: input config (modKey, mouse, layers)
#   - modes: modal system (desktop, theme — flat, all parented to app)
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
            exitAfter = mkOption {
              type = types.bool;
              default = false;
              description = ''
                Return to the root (app) mode immediately after this action
                runs. For launch/leaf actions (terminal, browser, launcher,
                lock) so you aren't stranded in the submap with keys eaten.
                Only meaningful inside submap modes; exec actions reset the
                submap first, then run the command.
              '';
            };
          };
        });
        default = { };
        description = "Named bindings in this mode";
      };
    };
  };

  # Dual-role layer type (engine-native): a trigger key that activates a mode.
  layerType = types.submodule {
    options = {
      hold = mkOption {
        type = types.nullOr types.str;
        default = null;
        description = ''
          The dual-role trigger key (e.g. "capslock"). Tapping it enters the
          target mode sticky; holding it enters the mode momentary.
        '';
      };
      entersMode = mkOption {
        type = types.nullOr types.str;
        default = null;
        description = ''
          The mode this trigger activates. The vogix input engine reads this
          directly to drive its mode statechart — tap = sticky (toggle), hold =
          momentary (left on release). No synthetic keysyms are involved.
        '';
      };
      tapHoldMs = mkOption {
        type = types.int;
        default = 250;
        description = "Tap↔hold threshold in milliseconds (tap = sticky, hold = momentary)";
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
                description = "Dual-role trigger layers (e.g. CapsLock → desktop), driven by the vogix input engine at the evdev level";
              };

              terminalClasses = mkOption {
                type = types.listOf types.str;
                default = [ ];
                description = ''
                  Hyprland window classes treated as terminals. When one is
                  focused, the Super→Ctrl remap is context-adjusted: copy/paste
                  retarget to Ctrl+Shift+C/V and the other remaps are suppressed,
                  so the macOS-style Super+C can't fire Ctrl+C = SIGINT into the
                  foreground job. Real Ctrl+C (no Super) still interrupts.
                '';
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
                description = "Desktop mode — focus, move, resize, workspaces, send-and-follow (single, unified WM mode)";
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
      };
    };
  };
}

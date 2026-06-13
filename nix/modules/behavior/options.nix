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
      stickyIdleMs = mkOption {
        type = types.int;
        default = 30000;
        description = "Idle milliseconds after which a sticky (tapped/locked) mode auto-reverts to root";
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

              paradigm = mkOption {
                type = types.str;
                default = "vogix";
                description = ''
                  Selected interaction paradigm — the GLOBAL, system-wide keybinding
                  model the input engine materializes. The catalog lives in the
                  engine (`src/input/catalog.rs`), NOT here: the engine resolves this
                  name into the paradigm's modes + mode graph and merges your own
                  launch/system/media OVERLAY (the `modes` below) on top.

                  Available: "vogix" (the house default — the user's own WM-nav
                  layout), "cua", "emacs", "i3", "vim", "windows", "macos", "linux".
                  Each is global: e.g. "cua" makes Ctrl+C copy everywhere, "macos"
                  applies the Cmd-feel Super→Ctrl remap, "i3"/"vim" drive the WM by
                  their own conventions.
                '';
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

              deviceFilter = mkOption {
                type = types.submodule {
                  options = {
                    excludeVendors = mkOption {
                      type = types.listOf types.int;
                      default = [ ];
                      description = "USB vendor ids the engine must NEVER grab (e.g. security keys).";
                    };
                    excludeNameSubstrings = mkOption {
                      type = types.listOf types.str;
                      default = [ ];
                      description = "Device-name substrings the engine must never grab (audio HID, consumer-control nodes).";
                    };
                  };
                };
                default = { };
                description = ''
                  Which evdev devices the input engine may OWN. The engine grabs
                  keyboards exclusively, so it must not grab a YubiKey (it would
                  break OTP/FIDO typing) or an audio mixer. A safe baseline is
                  baked into vogix (Yubico and audio HID are always excluded);
                  these lists ADD to it — they never replace it, so you can't
                  accidentally re-enable a security-key grab.
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

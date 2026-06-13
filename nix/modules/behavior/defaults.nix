# Vogix behavior defaults
#
# Two sub-domains:
#   keybindings: modKey, paradigm (selection), mouse, layers, terminalClasses
#   modes: app — the user's OVERLAY (launch/system/media); the paradigm WM-nav is
#          resolved by the engine, NOT encoded here.
#
# Philosophy:
#   - Keybindings are a flat catalog of interaction PARADIGMS (vim/emacs/mac/
#     windows/… + the house `vogix`), resolved by the input engine from the
#     `paradigm` selection. The paradigm provides the WM-navigation (focus / move
#     / resize / workspaces / window-state); the engine merges the user's own
#     OVERLAY (launch / system / media) on top. A paradigm is loaded ONCE and
#     every view (engine dispatch, `vogix input keys` help, the Hyprland fallback)
#     is materialized from it — nothing re-encodes the nav.
#   - `vogix` (the default) is the user's own layout: flat Super-combos, the
#     minimal Super+C/V copy/paste remap (`copy-paste`), no CapsLock. Its nav
#     lives in the engine (`src/input/catalog.rs::vogix_nav_preset`); only the
#     overlay below is authored in Nix (it uses keys praxis can't represent —
#     return/slash/print/XF86 — so it's the user's data, not the paradigm).
#
# The overlay binds also serve as the engine-off fallback (via the Hyprland
# generator): Super+Return (terminal) + the launcher are enough to recover and
# restart `vogix-input` if it ever fails to start.
_:

let
  # F12 system console: toggle the `console` Hyprland special workspace and lazily
  # launch a wezterm + tmux session in it. This is a plain exec (NOT an engine mode
  # switch) — the engine stays in `app`, which re-emits unbound keys, so typing
  # reaches tmux. Toggling again hides it; the `grep -q vogix-console ||` guard
  # avoids relaunching an already-running session.
  consoleToggleAction = "exec, hyprctl dispatch togglespecialworkspace console; hyprctl clients -j | grep -q vogix-console || wezterm start --class vogix-console -- tmux new-session -A -s console";
in
rec {
  # ── Input settings ──
  input = {
    repeatDelay = 200;
    sensitivity = -0.3;
    naturalScroll = true;
    leftHanded = true;
    floatSwitchOverrideFocus = 2;
    numlockByDefault = false;
  };

  # ── Touchpad ──
  touchpad = {
    naturalScroll = true;
    disableWhileTyping = true;
    scrollFactor = 0.3;
  };

  # ── Layout ──
  layout = "dwindle";

  layouts = {
    dwindle = {
      # NOTE: no `pseudotile` here — Hyprland removed the `dwindle:pseudotile`
      # config option (gone as of 0.55). Pseudotiling is now only the `pseudo`
      # dispatcher (a keybind), so the toggle would error
      # ("config option <dwindle:pseudotile> does not exist").
      preserve_split = true;
      force_split = 2;
      smart_resizing = true;
      use_active_for_splits = true;
    };
    master = {
      orientation = "center";
      special_scale_factor = 0.5;
    };
  };

  # ── Misc ──
  misc = {
    fontFamily = "Fira Code Nerd Font";
    disableLogo = true;
    disableAutoreload = false;
    alwaysFollowOnDnd = true;
    layersHogKeyboardFocus = true;
    animateManualResizes = true;
    enableSwallow = false;
    focusOnActivate = true;
  };

  # ── Gestures ──
  gestures = { };

  # ── Keybindings ──
  keybindings = {
    modKey = "super";

    # Interaction-paradigm SELECTION. `vogix` = the house default (the user's own
    # WM-nav layout). The engine resolves this name into the paradigm's nav modes
    # + mode graph and merges the overlay below; the catalog lives in the engine
    # (`src/input/catalog.rs`), not here. `vogix`'s remap is `copy-paste`
    # (Super+C/V → Ctrl+C/V, terminal-aware), supplied by the engine.
    paradigm = "vogix";

    # Window classes treated as terminals for the context-aware copy/paste remap:
    # there Super+C/V → Ctrl+Shift+C/V (so native Ctrl+C still SIGINTs), vs plain
    # Ctrl+C/V in GUI apps. POSIX termios: bare Ctrl+C in a terminal = SIGINT.
    terminalClasses = [
      "kitty"
      "org.wezfurlong.wezterm"
      "wezterm"
      "vogix-console"
      "Alacritty"
      "foot"
      "org.gnome.Console"
      "xterm-256color"
    ];

    # Mouse: Super+drag move, Super+right-drag resize (Hyprland `bindm`).
    mouse = {
      moveWindow = {
        button = "mouse:272";
        action = "movewindow";
        description = "Move window with mouse";
      };
      resizeWindow = {
        button = "mouse:273";
        action = "resizewindow";
        description = "Resize window with mouse";
      };
    };

    # No interaction layers — CapsLock is just CapsLock (no dual-role mode trigger).
    layers = { };
  };

  # ── Mode graph ── a single flat `app` mode for the overlay. The paradigm's mode
  # graph (and any sub-modes) is resolved by the engine; this is just the root the
  # overlay binds attach to (and what the Hyprland generator reads for its root +
  # console rules).
  modeGraph = {
    root = "app";
    modes = {
      app = { parent = null; type = "normal"; };
    };
  };

  # ── Modes ──
  modes = {
    # `app` holds ONLY the user's OVERLAY — launch / system / media bindings that
    # aren't the WM-navigation paradigm (and that use keys praxis can't express:
    # return/slash/print/XF86). The paradigm NAV (focus / move / resize /
    # workspaces / window-state) is resolved by the engine from
    # `keybindings.paradigm` and merged on top; it is NOT encoded here.
    app = {
      enter = null;
      exit = "escape";
      bindings = {
        # ── Launch ──
        terminal = { key = "super + return"; action = "exec, $TERMINAL"; description = "Terminal"; };
        browser = { key = "super + e"; action = "exec, $BROWSER"; description = "Browser"; };
        # Launcher + locker are environment choices, not vogix's — consume the
        # command from the environment (like $TERMINAL/$BROWSER above) instead of
        # hardcoding a tool. The host exports $LAUNCHER/$LOCKER; the `:-` fallback
        # keeps vogix usable standalone.
        launcher = { key = "super + space"; action = "exec, \${LAUNCHER:-walker}"; description = "Launcher"; };
        colorPicker = { key = "super + shift + p"; action = "exec, hyprpicker -a"; description = "Colour picker"; };
        lockScreen = { key = "super + shift + x"; action = "exec, \${LOCKER:-hyprlock}"; description = "Lock screen"; };

        # ── Screenshots ──
        # --cursor is invalid with the `area` target in current grimblast.
        screenshotClip = { key = "print"; action = "exec, grimblast --notify copy area"; description = "Screenshot → clipboard"; };
        screenshotEdit = { key = "shift + print"; action = "exec, grimblast save area - | swappy -f -"; description = "Screenshot → editor"; };

        # ── Gaps ──
        gapsOn = { key = "super + shift + g"; action = ''exec, hyprctl --batch "keyword general:gaps_out 5;keyword general:gaps_in 6"''; description = "Gaps on"; };
        gapsOff = { key = "super + g"; action = ''exec, hyprctl --batch "keyword general:gaps_out 0;keyword general:gaps_in 0"''; description = "Gaps off"; };

        # ── System (console, notifications, undo, help) ──
        console = { key = "F12"; action = consoleToggleAction; description = "Toggle system console"; };
        dismissNotification = { key = "super + d"; action = "exec, makoctl dismiss"; description = "Dismiss notification"; };
        dismissAll = { key = "super + shift + d"; action = "exec, makoctl dismiss --all"; description = "Dismiss all notifications"; };
        undoSession = { key = "super + z"; action = "exec, vogix session undo"; description = "Undo last window change"; };
        # Help is now an ENGINE view: it reads the resolved schema (paradigm nav +
        # this overlay) and renders it — replacing the build-time Nix help scripts.
        help = { key = "super + slash"; action = "exec, vogix input keys"; description = "Show keybindings"; };

        # ── Audio / brightness / media (XF86) ──
        volumeUp = { key = "XF86AudioRaiseVolume"; action = "exec, pamixer -i 5"; description = "Volume up"; };
        volumeDown = { key = "XF86AudioLowerVolume"; action = "exec, pamixer -d 5"; description = "Volume down"; };
        volumeMute = { key = "XF86AudioMute"; action = "exec, pamixer -t"; description = "Toggle mute"; };
        micMute = { key = "XF86AudioMicMute"; action = "exec, pamixer --default-source -t"; description = "Toggle mic"; };
        # `light` is not installed on this host; `brightnessctl` is.
        brightnessUp = { key = "XF86MonBrightnessUp"; action = "exec, brightnessctl set 5%+"; description = "Brighter"; };
        brightnessDown = { key = "XF86MonBrightnessDown"; action = "exec, brightnessctl set 5%-"; description = "Dimmer"; };
        mediaPlay = { key = "XF86AudioPlay"; action = "exec, playerctl play-pause"; description = "Play/pause"; };
        mediaNext = { key = "XF86AudioNext"; action = "exec, playerctl next"; description = "Next track"; };
        mediaPrev = { key = "XF86AudioPrev"; action = "exec, playerctl previous"; description = "Previous track"; };
      };
    };
  };
}

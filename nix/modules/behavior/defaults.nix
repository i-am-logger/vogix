# Vogix behavior defaults — flat Super-combo desktop UX
#
# Two sub-domains:
#   keybindings: modKey, mouse, paradigm (input config)
#   modes: app — a single, flat mode (no CapsLock, no sub-modes)
#
# Philosophy:
#   - One mode (`app`). Every WM action is a flat `Super`-combo, dispatched by the
#     vogix input engine to Hyprland. There is NO CapsLock mode, NO move/resize
#     sub-modes, and NO Super→Ctrl remap — Super is used directly as the WM
#     modifier, exactly like a plain Hyprland `bind =` config.
#   - The modifier on a direction picks the verb:
#       Super + direction        = focus        (h=left l=right j=up k=down)
#       Super + Shift + direction = move window  (swapwindow)
#       Ctrl  + Shift + direction = resize window (resizeactive)
#   - The `yuiop` window-state row:
#       Super+Q close · Super+Y float+pin · Super+F fullscreen · Super+P pseudo
#       Super+O togglesplit · Super+U togglegroup · Super+Tab cycle-group
#   - Workspaces: Super+1..0 (+ C=Chat, M=Music); Super+Ctrl+←/→/j/l switch;
#     Super+Ctrl+number send window there; Super+Shift+number send silently.
#
# Source: the user's own pre-vogix Hyprland config, carried verbatim — git
# `cce4ddc^:home/gui/hyprland/config/bindings.conf`. The engine stays (device-grab
# scope, hotplug, `vogix input doctor` observability); it simply loads this flat
# config. The earlier modal CapsLock model + macOS remap were vogix-era choices
# that have been removed. windows/mac/emacs paradigms are TODO — to be re-based as
# flat platform variants.
_:

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

    # Interaction PARADIGM (whole-WM flavour). `default` = the user's own config:
    # flat Super-combos, NO CapsLock, NO Super→Ctrl remap (remap = "none") — the
    # pre-vogix Hyprland workflow carried into the engine. windows/mac/emacs are
    # TODO: to be re-based as flat platform variants (the modal versions were
    # removed with the CapsLock model).
    paradigm = "default";
    paradigms = {
      default = {
        remap = "none";
        inherit modes;
      };
    };

    # Window classes treated as terminals for context-aware remaps. Unused while
    # remap = "none", kept as data for when a remapping paradigm is added.
    terminalClasses = [
      "kitty"
      "org.wezfurlong.wezterm"
      "wezterm"
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

  # ── Mode graph ── a SINGLE flat mode. No sub-modes.
  # Axioms (praxis applied/hmi/input/modes.rs): NoDeadStates, RootReachable,
  # RootNoParent — trivially satisfied by one normal root mode.
  modeGraph = {
    root = "app";
    modes = {
      app = { parent = null; type = "normal"; };
    };
  };

  # ── Modes ──
  modes = {
    # The one mode: apps + window management, all flat Super-combos. Carried
    # verbatim from the user's pre-vogix Hyprland config
    # (git cce4ddc^:home/gui/hyprland/config/bindings.conf).
    app = {
      enter = null;
      exit = "escape";
      bindings = {
        # ── Launch ──
        terminal = { key = "super + return"; action = "exec, $TERMINAL"; description = "Terminal"; };
        browser = { key = "super + e"; action = "exec, $BROWSER"; description = "Browser"; };
        chrome = { key = "super + shift + e"; action = "exec, google-chrome-stable"; description = "Chrome"; };
        launcher = { key = "super + space"; action = "exec, walker -p 'Start…' -w 1000 -h 700"; description = "Launcher"; };
        colorPicker = { key = "super + shift + p"; action = "exec, hyprpicker -a"; description = "Colour picker"; };
        lockScreen = { key = "super + shift + x"; action = "exec, hyprlock"; description = "Lock screen"; };

        # ── Screenshots ──
        # --cursor is invalid with the `area` target in current grimblast
        # ("'-c|--cursor' cannot be used with TARGET 'area'") — region shots omit it.
        screenshotClip = { key = "print"; action = "exec, grimblast --notify copy area"; description = "Screenshot → clipboard"; };
        screenshotEdit = { key = "shift + print"; action = "exec, grimblast save area - | swappy -f -"; description = "Screenshot → editor"; };

        # ── Window state (the yuiop row + q/f/tab/gaps) ──
        closeWindow = { key = "super + q"; action = "killactive,"; description = "Close window"; };
        floatPin = { key = "super + y"; action = "exec, hyprctl dispatch togglefloating ; hyprctl dispatch pin"; description = "Float + pin"; };
        fullscreen = { key = "super + f"; action = "fullscreen"; description = "Fullscreen"; };
        pseudo = { key = "super + p"; action = "pseudo,"; description = "Pseudotile"; };
        # togglesplit is a dwindle layout message, not a top-level dispatcher
        # (the bare `togglesplit` dispatcher was removed from Hyprland).
        toggleSplit = { key = "super + o"; action = "layoutmsg, togglesplit"; description = "Toggle split"; };
        toggleGroup = { key = "super + u"; action = "togglegroup,"; description = "Toggle group"; };
        groupCycle = { key = "super + tab"; action = "changegroupactive, f"; description = "Cycle window in group"; };
        gapsOn = { key = "super + shift + g"; action = ''exec, hyprctl --batch "keyword general:gaps_out 5;keyword general:gaps_in 6"''; description = "Gaps on"; };
        gapsOff = { key = "super + g"; action = ''exec, hyprctl --batch "keyword general:gaps_out 0;keyword general:gaps_in 0"''; description = "Gaps off"; };

        # ── Focus (Super + direction; j = up, k = down — non-vim, your original) ──
        focusLeft = { key = "super + h"; action = "movefocus, l"; description = "Focus left"; };
        focusRight = { key = "super + l"; action = "movefocus, r"; description = "Focus right"; };
        focusUp = { key = "super + j"; action = "movefocus, u"; description = "Focus up"; };
        focusDown = { key = "super + k"; action = "movefocus, d"; description = "Focus down"; };
        focusLeftArrow = { key = "super + left"; action = "movefocus, l"; description = "Focus left"; };
        focusRightArrow = { key = "super + right"; action = "movefocus, r"; description = "Focus right"; };
        focusUpArrow = { key = "super + up"; action = "movefocus, u"; description = "Focus up"; };
        focusDownArrow = { key = "super + down"; action = "movefocus, d"; description = "Focus down"; };

        # ── Move window (Super + Shift + direction → swapwindow) ──
        swapLeft = { key = "super + shift + h"; action = "swapwindow, l"; description = "Move window left"; };
        swapRight = { key = "super + shift + l"; action = "swapwindow, r"; description = "Move window right"; };
        swapUp = { key = "super + shift + j"; action = "swapwindow, u"; description = "Move window up"; };
        swapDown = { key = "super + shift + k"; action = "swapwindow, d"; description = "Move window down"; };
        swapLeftArrow = { key = "super + shift + left"; action = "swapwindow, l"; description = "Move window left"; };
        swapRightArrow = { key = "super + shift + right"; action = "swapwindow, r"; description = "Move window right"; };
        swapUpArrow = { key = "super + shift + up"; action = "swapwindow, u"; description = "Move window up"; };
        swapDownArrow = { key = "super + shift + down"; action = "swapwindow, d"; description = "Move window down"; };

        # ── Resize window (Ctrl + Shift + direction → resizeactive; repeats) ──
        resizeLeft = { key = "ctrl + shift + h"; action = "resizeactive, -30 0"; description = "Narrower"; repeat = true; };
        resizeRight = { key = "ctrl + shift + l"; action = "resizeactive, 30 0"; description = "Wider"; repeat = true; };
        resizeUp = { key = "ctrl + shift + j"; action = "resizeactive, 0 -30"; description = "Shorter"; repeat = true; };
        resizeDown = { key = "ctrl + shift + k"; action = "resizeactive, 0 30"; description = "Taller"; repeat = true; };
        resizeLeftArrow = { key = "ctrl + shift + left"; action = "resizeactive, -30 0"; description = "Narrower"; repeat = true; };
        resizeRightArrow = { key = "ctrl + shift + right"; action = "resizeactive, 30 0"; description = "Wider"; repeat = true; };
        resizeUpArrow = { key = "ctrl + shift + up"; action = "resizeactive, 0 -30"; description = "Shorter"; repeat = true; };
        resizeDownArrow = { key = "ctrl + shift + down"; action = "resizeactive, 0 30"; description = "Taller"; repeat = true; };

        # ── Workspaces (Super + number; C = Chat, M = Music) ──
        workspace1 = { key = "super + 1"; action = "workspace, 1"; description = "Workspace 1"; };
        workspace2 = { key = "super + 2"; action = "workspace, 2"; description = "Workspace 2"; };
        workspace3 = { key = "super + 3"; action = "workspace, 3"; description = "Workspace 3"; };
        workspace4 = { key = "super + 4"; action = "workspace, 4"; description = "Workspace 4"; };
        workspace5 = { key = "super + 5"; action = "workspace, 5"; description = "Workspace 5"; };
        workspace6 = { key = "super + 6"; action = "workspace, 6"; description = "Workspace 6"; };
        workspace7 = { key = "super + 7"; action = "workspace, 7"; description = "Workspace 7"; };
        workspace8 = { key = "super + 8"; action = "workspace, 8"; description = "Workspace 8"; };
        workspace9 = { key = "super + 9"; action = "workspace, 9"; description = "Workspace 9"; };
        workspace10 = { key = "super + 0"; action = "workspace, 10"; description = "Workspace 10"; };
        workspaceChat = { key = "super + c"; action = "workspace, Chat"; description = "Chat workspace"; };
        workspaceMusic = { key = "super + m"; action = "workspace, Music"; description = "Music workspace"; };

        # ── Adjacent workspace (Super + Ctrl + ←/→ or j/l) ──
        wsPrev = { key = "super + ctrl + left"; action = "workspace, -1"; description = "Previous workspace"; };
        wsNext = { key = "super + ctrl + right"; action = "workspace, +1"; description = "Next workspace"; };
        wsPrevJ = { key = "super + ctrl + j"; action = "workspace, -1"; description = "Previous workspace"; };
        wsNextL = { key = "super + ctrl + l"; action = "workspace, +1"; description = "Next workspace"; };

        # ── Send window to workspace (Super + Ctrl + number) ──
        moveToWs1 = { key = "super + ctrl + 1"; action = "movetoworkspace, 1"; description = "Send window to workspace 1"; };
        moveToWs2 = { key = "super + ctrl + 2"; action = "movetoworkspace, 2"; description = "Send window to workspace 2"; };
        moveToWs3 = { key = "super + ctrl + 3"; action = "movetoworkspace, 3"; description = "Send window to workspace 3"; };
        moveToWs4 = { key = "super + ctrl + 4"; action = "movetoworkspace, 4"; description = "Send window to workspace 4"; };
        moveToWs5 = { key = "super + ctrl + 5"; action = "movetoworkspace, 5"; description = "Send window to workspace 5"; };
        moveToWs6 = { key = "super + ctrl + 6"; action = "movetoworkspace, 6"; description = "Send window to workspace 6"; };
        moveToWs7 = { key = "super + ctrl + 7"; action = "movetoworkspace, 7"; description = "Send window to workspace 7"; };
        moveToWs8 = { key = "super + ctrl + 8"; action = "movetoworkspace, 8"; description = "Send window to workspace 8"; };
        moveToWs9 = { key = "super + ctrl + 9"; action = "movetoworkspace, 9"; description = "Send window to workspace 9"; };
        moveToWs10 = { key = "super + ctrl + 0"; action = "movetoworkspace, 10"; description = "Send window to workspace 10"; };

        # ── Send window to adjacent workspace (Super + Ctrl + Shift + ←/→ or j/l) ──
        sendWsPrev = { key = "super + ctrl + shift + left"; action = "movetoworkspace, -1"; description = "Send window ← workspace"; };
        sendWsNext = { key = "super + ctrl + shift + right"; action = "movetoworkspace, +1"; description = "Send window → workspace"; };
        sendWsPrevJ = { key = "super + ctrl + shift + j"; action = "movetoworkspace, -1"; description = "Send window ← workspace"; };
        sendWsNextL = { key = "super + ctrl + shift + l"; action = "movetoworkspace, +1"; description = "Send window → workspace"; };

        # ── Send window to workspace silently (Super + Shift + number) ──
        moveSilent1 = { key = "super + shift + 1"; action = "movetoworkspacesilent, 1"; description = "Send window to workspace 1 (silent)"; };
        moveSilent2 = { key = "super + shift + 2"; action = "movetoworkspacesilent, 2"; description = "Send window to workspace 2 (silent)"; };
        moveSilent3 = { key = "super + shift + 3"; action = "movetoworkspacesilent, 3"; description = "Send window to workspace 3 (silent)"; };
        moveSilent4 = { key = "super + shift + 4"; action = "movetoworkspacesilent, 4"; description = "Send window to workspace 4 (silent)"; };
        moveSilent5 = { key = "super + shift + 5"; action = "movetoworkspacesilent, 5"; description = "Send window to workspace 5 (silent)"; };
        moveSilent6 = { key = "super + shift + 6"; action = "movetoworkspacesilent, 6"; description = "Send window to workspace 6 (silent)"; };
        moveSilent7 = { key = "super + shift + 7"; action = "movetoworkspacesilent, 7"; description = "Send window to workspace 7 (silent)"; };
        moveSilent8 = { key = "super + shift + 8"; action = "movetoworkspacesilent, 8"; description = "Send window to workspace 8 (silent)"; };
        moveSilent9 = { key = "super + shift + 9"; action = "movetoworkspacesilent, 9"; description = "Send window to workspace 9 (silent)"; };
        moveSilent10 = { key = "super + shift + 0"; action = "movetoworkspacesilent, 10"; description = "Send window to workspace 10 (silent)"; };

        # ── Audio / brightness / media (XF86) ──
        volumeUp = { key = "XF86AudioRaiseVolume"; action = "exec, pamixer -i 5"; description = "Volume up"; };
        volumeDown = { key = "XF86AudioLowerVolume"; action = "exec, pamixer -d 5"; description = "Volume down"; };
        volumeMute = { key = "XF86AudioMute"; action = "exec, pamixer -t"; description = "Toggle mute"; };
        micMute = { key = "XF86AudioMicMute"; action = "exec, pamixer --default-source -t"; description = "Toggle mic"; };
        brightnessUp = { key = "XF86MonBrightnessUp"; action = "exec, light -A 5"; description = "Brighter"; };
        brightnessDown = { key = "XF86MonBrightnessDown"; action = "exec, light -U 5"; description = "Dimmer"; };
        mediaPlay = { key = "XF86AudioPlay"; action = "exec, playerctl play-pause"; description = "Play/pause"; };
        mediaNext = { key = "XF86AudioNext"; action = "exec, playerctl next"; description = "Next track"; };
        mediaPrev = { key = "XF86AudioPrev"; action = "exec, playerctl previous"; description = "Previous track"; };
      };
    };
  };
}

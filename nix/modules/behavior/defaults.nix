# Vogix behavior defaults — Modal desktop UX
#
# Two sub-domains:
#   keybindings: modKey, mouse, layers (input config)
#   modes: app, desktop, arrange, theme (contextual actions)
#
# Philosophy:
#   - App mode (default): Super = Command (macOS-like), keys → apps
#   - Desktop mode (Super+Escape / CapsLock toggle): single keys for WM
#   - Arrange mode (from desktop): move + resize windows
#   - Theme mode (from desktop): vogix appearance switching
#
# Semantic keys (consistent across modes):
#   h/j/k/l = directional, q = quit, f = fullscreen, y = float (yank),
#   o = split, a = arrange, s = swap, d = dismiss, n/p = next/prev,
#   v = vogix, x = lock, Space = command palette
_:

{
  # ── Keybindings (input config) ──
  keybindings = {
    modKey = "super";

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

    layers = {
      # CapsLock → Scroll_Lock (toggles desktop mode via Hyprland)
      desktopToggle = {
        hold = "capslock";
        tapAction = "slck";
        tapHoldMs = 1; # Effectively always tap — no hold behavior for now
        bindings = { };
      };
    };
  };

  # Super → Ctrl remaps (internal, derived from modKey = "super")
  # Not user-facing — generated automatically when modKey is super
  _superCtrlRemaps = {
    copy = { from = "super + c"; to = "ctrl + c"; };
    paste = { from = "super + v"; to = "ctrl + v"; };
    cut = { from = "super + x"; to = "ctrl + x"; };
    undo = { from = "super + z"; to = "ctrl + z"; };
    save = { from = "super + s"; to = "ctrl + s"; };
    selectAll = { from = "super + a"; to = "ctrl + a"; };
    find = { from = "super + f"; to = "ctrl + f"; };
    closeTab = { from = "super + w"; to = "ctrl + w"; };
    newTab = { from = "super + t"; to = "ctrl + t"; };
    newWindow = { from = "super + n"; to = "ctrl + n"; };
    print = { from = "super + p"; to = "ctrl + p"; };
    reload = { from = "super + r"; to = "ctrl + r"; };
    addressBar = { from = "super + l"; to = "ctrl + l"; };
    open = { from = "super + o"; to = "ctrl + o"; };
    bold = { from = "super + b"; to = "ctrl + b"; };
    italic = { from = "super + i"; to = "ctrl + i"; };
    underline = { from = "super + u"; to = "ctrl + u"; };
    redo = { from = "super + y"; to = "ctrl + y"; };
    quit = { from = "super + q"; to = "ctrl + q"; };
    goToLine = { from = "super + g"; to = "ctrl + g"; };
    devTools = { from = "super + d"; to = "ctrl + d"; };
  };

  # ── Modes (contextual actions) ──
  modes = {
    # App mode: always-available bindings (workspaces, media, mode entry)
    app = {
      enter = null;
      exit = "escape";
      bindings = {
        # ── Workspaces ──
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

        # ── Quick access ──
        terminal = { key = "super + return"; action = "exec, $TERMINAL"; description = "Terminal"; };
        launcher = { key = "super + space"; action = "exec, walker -p 'Start…' -w 1000 -h 700"; description = "Launcher"; };

        # ── Audio ──
        volumeUp = { key = "XF86AudioRaiseVolume"; action = "exec, pamixer -i 5"; description = "Volume up"; };
        volumeDown = { key = "XF86AudioLowerVolume"; action = "exec, pamixer -d 5"; description = "Volume down"; };
        volumeMute = { key = "XF86AudioMute"; action = "exec, pamixer -t"; description = "Toggle mute"; };
        micMute = { key = "XF86AudioMicMute"; action = "exec, pamixer --default-source -t"; description = "Toggle mic"; };

        # ── Screen brightness ──
        brightnessUp = { key = "XF86MonBrightnessUp"; action = "exec, light -A 5"; description = "Screen brighter"; };
        brightnessDown = { key = "XF86MonBrightnessDown"; action = "exec, light -U 5"; description = "Screen dimmer"; };

        # ── Peripheral brightness (OpenRGB) ──
        peripheralBrightnessUp = { key = "XF86KbdBrightnessUp"; action = "exec, openrgb --brightness +10"; description = "Peripherals brighter"; };
        peripheralBrightnessDown = { key = "XF86KbdBrightnessDown"; action = "exec, openrgb --brightness -10"; description = "Peripherals dimmer"; };

        # ── Media ──
        mediaPlay = { key = "XF86AudioPlay"; action = "exec, playerctl play-pause"; description = "Play/pause"; };
        mediaNext = { key = "XF86AudioNext"; action = "exec, playerctl next"; description = "Next track"; };
        mediaPrev = { key = "XF86AudioPrev"; action = "exec, playerctl previous"; description = "Previous track"; };

        # ── Screenshot ──
        screenshotClip = { key = "print"; action = "exec, grimblast --notify copy area"; description = "Screenshot → clipboard"; };
        screenshotEdit = { key = "shift + print"; action = "exec, grimblast save area - | swappy -f -"; description = "Screenshot → editor"; };

        # ── Help ──
        help = { key = "super + slash"; action = "exec, vogix-modes-global"; description = "Show keybindings"; };

        # ── Desktop mode entry ──
        enterDesktop = { key = "Scroll_Lock"; action = "submap, desktop"; description = "Enter desktop mode (CapsLock tap)"; };
        enterDesktopFallback = { key = "super + escape"; action = "submap, desktop"; description = "Enter desktop mode"; };
      };
    };

    # Desktop mode: manage environment with single keys
    desktop = {
      enter = null;
      exit = "escape";
      bindings = {
        focusLeft = { key = "h"; action = "movefocus, l"; description = "Focus left"; };
        focusDown = { key = "j"; action = "movefocus, d"; description = "Focus down"; };
        focusUp = { key = "k"; action = "movefocus, u"; description = "Focus up"; };
        focusRight = { key = "l"; action = "movefocus, r"; description = "Focus right"; };
        focusLeftArrow = { key = "left"; action = "movefocus, l"; description = "Focus left"; };
        focusDownArrow = { key = "down"; action = "movefocus, d"; description = "Focus down"; };
        focusUpArrow = { key = "up"; action = "movefocus, u"; description = "Focus up"; };
        focusRightArrow = { key = "right"; action = "movefocus, r"; description = "Focus right"; };

        workspace1 = { key = "1"; action = "workspace, 1"; description = "Workspace 1"; };
        workspace2 = { key = "2"; action = "workspace, 2"; description = "Workspace 2"; };
        workspace3 = { key = "3"; action = "workspace, 3"; description = "Workspace 3"; };
        workspace4 = { key = "4"; action = "workspace, 4"; description = "Workspace 4"; };
        workspace5 = { key = "5"; action = "workspace, 5"; description = "Workspace 5"; };
        workspace6 = { key = "6"; action = "workspace, 6"; description = "Workspace 6"; };
        workspace7 = { key = "7"; action = "workspace, 7"; description = "Workspace 7"; };
        workspace8 = { key = "8"; action = "workspace, 8"; description = "Workspace 8"; };
        workspace9 = { key = "9"; action = "workspace, 9"; description = "Workspace 9"; };
        workspace10 = { key = "0"; action = "workspace, 10"; description = "Workspace 10"; };
        workspaceNext = { key = "n"; action = "workspace, +1"; description = "Next workspace"; };
        workspacePrev = { key = "p"; action = "workspace, -1"; description = "Previous workspace"; };

        closeWindow = { key = "q"; action = "killactive,"; description = "Close window"; };
        fullscreen = { key = "f"; action = "fullscreen"; description = "Fullscreen"; };
        toggleFloat = { key = "y"; action = "togglefloating,"; description = "Float (yank from tiling)"; };
        toggleSplit = { key = "o"; action = "layoutmsg, togglesplit"; description = "Toggle split"; };

        openTerminal = { key = "t"; action = "exec, $TERMINAL"; description = "Terminal"; };
        openBrowser = { key = "e"; action = "exec, $BROWSER"; description = "Browser"; };
        openLauncher = { key = "space"; action = "exec, walker -p 'Start…' -w 1000 -h 700"; description = "Launcher"; };

        dismissNotification = { key = "d"; action = "exec, makoctl dismiss"; description = "Dismiss notification"; };
        dismissAll = { key = "shift + d"; action = "exec, makoctl dismiss --all"; description = "Dismiss all"; };

        lock = { key = "x"; action = "exec, hyprlock"; description = "Lock screen"; };

        enterArrange = { key = "a"; action = "submap, arrange"; description = "Arrange mode"; };
        enterTheme = { key = "v"; action = "submap, theme"; description = "Theme mode (vogix)"; };

        help = { key = "slash"; action = "exec, vogix-modes-desktop"; description = "Show keybindings"; };

        exitDesktop = { key = "Scroll_Lock"; action = "submap, reset"; description = "Back to app mode"; };
      };
    };

    # Arrange mode: move + resize windows
    arrange = {
      enter = "a";
      exit = "escape";
      bindings = {
        moveLeft = { key = "h"; action = "movewindow, l"; description = "Move left"; };
        moveDown = { key = "j"; action = "movewindow, d"; description = "Move down"; };
        moveUp = { key = "k"; action = "movewindow, u"; description = "Move up"; };
        moveRight = { key = "l"; action = "movewindow, r"; description = "Move right"; };
        moveLeftArrow = { key = "left"; action = "movewindow, l"; description = "Move left"; };
        moveDownArrow = { key = "down"; action = "movewindow, d"; description = "Move down"; };
        moveUpArrow = { key = "up"; action = "movewindow, u"; description = "Move up"; };
        moveRightArrow = { key = "right"; action = "movewindow, r"; description = "Move right"; };

        resizeLeft = { key = "shift + h"; action = "resizeactive, -30 0"; description = "Shrink width"; repeat = true; };
        resizeDown = { key = "shift + j"; action = "resizeactive, 0 30"; description = "Grow height"; repeat = true; };
        resizeUp = { key = "shift + k"; action = "resizeactive, 0 -30"; description = "Shrink height"; repeat = true; };
        resizeRight = { key = "shift + l"; action = "resizeactive, 30 0"; description = "Grow width"; repeat = true; };
        resizeLeftArrow = { key = "shift + left"; action = "resizeactive, -30 0"; description = "Shrink width"; repeat = true; };
        resizeDownArrow = { key = "shift + down"; action = "resizeactive, 0 30"; description = "Grow height"; repeat = true; };
        resizeUpArrow = { key = "shift + up"; action = "resizeactive, 0 -30"; description = "Shrink height"; repeat = true; };
        resizeRightArrow = { key = "shift + right"; action = "resizeactive, 30 0"; description = "Grow width"; repeat = true; };

        sendToWorkspace1 = { key = "1"; action = "movetoworkspace, 1"; description = "Send to workspace 1"; };
        sendToWorkspace2 = { key = "2"; action = "movetoworkspace, 2"; description = "Send to workspace 2"; };
        sendToWorkspace3 = { key = "3"; action = "movetoworkspace, 3"; description = "Send to workspace 3"; };
        sendToWorkspace4 = { key = "4"; action = "movetoworkspace, 4"; description = "Send to workspace 4"; };
        sendToWorkspace5 = { key = "5"; action = "movetoworkspace, 5"; description = "Send to workspace 5"; };
        sendToWorkspace6 = { key = "6"; action = "movetoworkspace, 6"; description = "Send to workspace 6"; };
        sendToWorkspace7 = { key = "7"; action = "movetoworkspace, 7"; description = "Send to workspace 7"; };
        sendToWorkspace8 = { key = "8"; action = "movetoworkspace, 8"; description = "Send to workspace 8"; };
        sendToWorkspace9 = { key = "9"; action = "movetoworkspace, 9"; description = "Send to workspace 9"; };
        sendToWorkspace10 = { key = "0"; action = "movetoworkspace, 10"; description = "Send to workspace 10"; };

        fullscreen = { key = "f"; action = "fullscreen"; description = "Fullscreen"; };
        toggleFloat = { key = "y"; action = "togglefloating,"; description = "Float"; };
        toggleSplit = { key = "o"; action = "layoutmsg, togglesplit"; description = "Toggle split"; };
        swap = { key = "s"; action = "swapnext,"; description = "Swap with neighbor"; };

        help = { key = "slash"; action = "exec, vogix-modes-arrange"; description = "Show keybindings"; };
      };
    };

    # Theme mode: vogix appearance switching
    theme = {
      enter = "v";
      exit = "escape";
      bindings = {
        nextTheme = { key = "n"; action = "exec, vogix -t next"; description = "Next theme"; };
        prevTheme = { key = "p"; action = "exec, vogix -t prev"; description = "Previous theme"; };
        darker = { key = "d"; action = "exec, vogix -v darker"; description = "Darker variant"; };
        lighter = { key = "l"; action = "exec, vogix -v lighter"; description = "Lighter variant"; };
        cycleScheme = { key = "s"; action = "exec, vogix -s next"; description = "Cycle scheme"; };

        screenBrighterTheme = { key = "XF86MonBrightnessUp"; action = "exec, vogix -v lighter"; description = "Lighter variant"; };
        screenDimmerTheme = { key = "XF86MonBrightnessDown"; action = "exec, vogix -v darker"; description = "Darker variant"; };
        peripheralBrighterTheme = { key = "XF86KbdBrightnessUp"; action = "exec, openrgb --brightness +10"; description = "Peripherals brighter"; };
        peripheralDimmerTheme = { key = "XF86KbdBrightnessDown"; action = "exec, openrgb --brightness -10"; description = "Peripherals dimmer"; };

        showStatus = { key = "space"; action = "exec, vogix status | xargs notify-send 'Vogix'"; description = "Show current theme"; };

        help = { key = "slash"; action = "exec, vogix-modes-theme"; description = "Show keybindings"; };
      };
    };
  };
}

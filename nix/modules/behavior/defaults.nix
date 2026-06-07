# Vogix behavior defaults — Modal desktop UX
#
# Two sub-domains:
#   keybindings: modKey, mouse, layers (input config)
#   modes: app, desktop (+ console infra) — a single, unified WM mode
#
# Philosophy:
#   - App mode (default): Super = Command (macOS-like), keys → apps
#   - Desktop mode: the one WM mode, dual-role CapsLock. HOLD CapsLock = momentary
#     (acts while held, reverts on release — Raskin's quasimode). TAP CapsLock =
#     sticky/locked (stays until you tap again; a forgotten sticky self-heals via
#     idle auto-revert). The active mode is shown by the window border colour, and
#     Esc is the always-available safety-net exit.
#   - Console: F12 tmux overlay (infra, passthrough).
#
# Desktop keys — the modifier on a direction picks the verb:
#   direction (arrows or h/j/k/l) = focus
#   Shift + direction             = move window
#   Ctrl  + direction             = resize window
#   number                        = go to workspace
#   Shift + number                = send window to workspace + follow
#   q = close, f = fullscreen, y = float, x = lock, d = dismiss, Space = launcher
_:

let
  # Console toggle command — used in every mode that supports F12 console access.
  # Checks if console workspace is visible, starts wezterm+tmux if needed, switches submap.
  consoleToggleAction = "exec, hyprctl dispatch togglespecialworkspace console; if hyprctl monitors -j | grep -q special:console; then hyprctl clients -j | grep -q vogix-console || wezterm start --class vogix-console -- tmux new-session -A -s console; hyprctl dispatch submap console; else hyprctl dispatch submap reset; fi";
in
# `rec` so a paradigm preset can reference the shared `modes`/`modeGraph` below.
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

  # ── Keybindings (modal input) ──
  keybindings = {
    modKey = "super";
    # Interaction PARADIGM (whole-WM flavour) the user selects. Each paradigm in
    # `paradigms` below pairs a Super-modifier remap (a praxis RemapSet preset)
    # with a per-mode WM-navigation binding set. `mkSchemaJSON` resolves the
    # chosen paradigm into the engine's `modes` + remap. Default `default` = the
    # user's own preferred config (modal CapsLock→desktop bare-key style).
    paradigm = "default";

    # WM interaction paradigms. A paradigm = { remap = <praxis RemapSet name>;
    # modes = <per-mode bindings over vogix's own app/desktop/move/resize>; }.
    # The user picks one via `paradigm` above and overlays their own bindings via
    # `programs.vogix.behavior.modes` (recursiveUpdate, per binding name).
    paradigms = {
      # default: the user's own preferred model — modal CapsLock→desktop, bare
      # hjkl/arrows. Its bindings ARE the shared `modes` below, so it is the identity.
      default = {
        remap = "macos";
        inherit modes;
      };

      # windows: chorded Win-key navigation IN app mode (no CapsLock needed),
      # remap = none (Windows uses Ctrl natively for copy/paste). Reuses vim's app
      # bindings (workspaces/media/console) + adds Super-combo WM nav; the CapsLock
      # modal layer (desktop/move/resize) stays available as a bonus.
      windows = {
        remap = "none";
        modes = {
          app = {
            enter = null;
            exit = "escape";
            bindings = modes.app.bindings // {
              winFocusLeft = { key = "super + left"; action = "movefocus, l"; description = "Focus left"; repeat = true; };
              winFocusDown = { key = "super + down"; action = "movefocus, d"; description = "Focus down"; repeat = true; };
              winFocusUp = { key = "super + up"; action = "movefocus, u"; description = "Focus up"; repeat = true; };
              winFocusRight = { key = "super + right"; action = "movefocus, r"; description = "Focus right"; repeat = true; };
              winMoveLeft = { key = "super + shift + left"; action = "movewindow, l"; description = "Move window left"; repeat = true; };
              winMoveDown = { key = "super + shift + down"; action = "movewindow, d"; description = "Move window down"; repeat = true; };
              winMoveUp = { key = "super + shift + up"; action = "movewindow, u"; description = "Move window up"; repeat = true; };
              winMoveRight = { key = "super + shift + right"; action = "movewindow, r"; description = "Move window right"; repeat = true; };
              winResizeLeft = { key = "super + ctrl + left"; action = "resizeactive, -40 0"; description = "Shrink width"; repeat = true; };
              winResizeRight = { key = "super + ctrl + right"; action = "resizeactive, 40 0"; description = "Grow width"; repeat = true; };
              winResizeUp = { key = "super + ctrl + up"; action = "resizeactive, 0 -40"; description = "Shrink height"; repeat = true; };
              winResizeDown = { key = "super + ctrl + down"; action = "resizeactive, 0 40"; description = "Grow height"; repeat = true; };
              winCycle = { key = "alt + tab"; action = "cyclenext,"; description = "Cycle windows"; };
              winClose = { key = "super + q"; action = "killactive,"; description = "Close window"; };
              winCloseAltF4 = { key = "alt + F4"; action = "killactive,"; description = "Close window"; };
              winFullscreen = { key = "super + f"; action = "fullscreen, 0"; description = "Fullscreen"; };
              winFloat = { key = "super + shift + space"; action = "togglefloating,"; description = "Toggle floating"; };
            };
          };
          inherit (modes) desktop move resize console;
        };
      };

      # mac: chorded Command-key navigation; keeps the macOS Super→Ctrl remap for
      # app shortcuts (Cmd+C/V/Q/W → Ctrl…), so WM nav avoids Super+letter and uses
      # Super+arrows / Super+Tab / Cmd+Ctrl+F (the real macOS fullscreen). CapsLock
      # modal layer stays available.
      mac = {
        remap = "macos";
        modes = {
          app = {
            enter = null;
            exit = "escape";
            bindings = modes.app.bindings // {
              macFocusLeft = { key = "super + left"; action = "movefocus, l"; description = "Focus left"; repeat = true; };
              macFocusDown = { key = "super + down"; action = "movefocus, d"; description = "Focus down"; repeat = true; };
              macFocusUp = { key = "super + up"; action = "movefocus, u"; description = "Focus up"; repeat = true; };
              macFocusRight = { key = "super + right"; action = "movefocus, r"; description = "Focus right"; repeat = true; };
              macMoveLeft = { key = "super + shift + left"; action = "movewindow, l"; description = "Move window left"; repeat = true; };
              macMoveDown = { key = "super + shift + down"; action = "movewindow, d"; description = "Move window down"; repeat = true; };
              macMoveUp = { key = "super + shift + up"; action = "movewindow, u"; description = "Move window up"; repeat = true; };
              macMoveRight = { key = "super + shift + right"; action = "movewindow, r"; description = "Move window right"; repeat = true; };
              macCycle = { key = "super + tab"; action = "cyclenext,"; description = "Cycle windows"; };
              macFullscreen = { key = "super + ctrl + f"; action = "fullscreen, 0"; description = "Fullscreen (Cmd+Ctrl+F)"; };
            };
          };
          inherit (modes) desktop move resize console;
        };
      };
    };

    # Window classes treated as terminals for the context-aware Super→Ctrl remap
    # (copy/paste → Ctrl+Shift+C/V; other remaps suppressed). Loaded as data —
    # override per host. POSIX termios: bare Ctrl+C in a terminal = SIGINT.
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
      # ── CapsLock interaction model (the single source of truth) ──
      #
      #   caps + a WM key    = MOMENTARY: do the action, return to app when you
      #                        let go of caps. (caps+→ nudge, caps+q close, …)
      #   caps clicked alone = STICKY: stay in desktop for several actions;
      #                        click caps again to leave.
      #   Esc                = safety net; always returns to app.
      #
      # The vogix input engine owns this directly in one process: a dual-role
      # CapsLock detector (tap vs hold, threshold = tapHoldMs, "tap-hold-press"
      # so caps+key is always momentary) drives the praxis mode statechart and
      # dispatches WM actions over the Hyprland control socket. `entersMode` names
      # the mode CapsLock activates — a tap enters it sticky, a hold enters it
      # momentary (released on caps-up). No synthetic keysyms and no compositor
      # submaps are involved. Per-binding exit semantics live on each binding via
      # `exitAfter` (true = return to app after a one-shot; false = stay/chain).
      desktopToggle = {
        hold = "capslock";
        entersMode = "desktop";
        tapHoldMs = 250; # lone press released within this = tap (sticky); else hold
        stickyIdleMs = 30000; # a tapped (locked) mode self-reverts after this idle
      };
    };
  };

  # ── Mode graph — defines mode topology ──
  # Mirrors the ModeGraph ontology in praxis applied/hmi/input/modes.rs
  # root: the default mode (Hyprland "reset" submap)
  # modes: each mode's parent (exit target) and type
  # Axioms: NoDeadStates, RootReachable, RootNoParent
  #
  # Design: a SINGLE WM mode (`desktop`). The old `arrange` and `theme`
  # sub-modes were removed entirely:
  #   - arrange folded into desktop — focus = dir, Shift+dir = move,
  #     Ctrl+dir = resize, Shift+number = send-and-follow. No sub-mode hop.
  #   - theme dropped — it was used once in two weeks of telemetry; appearance
  #     switching lives in the `vogix` CLI / brightness keys instead.
  #
  # Entering/leaving desktop is dual-role on CapsLock — see the full model on
  # layers.desktopToggle above. In short: caps+key = momentary (exit on
  # release), caps clicked alone = sticky toggle, Esc = safety net only.
  # move/resize are flat sub-modes (parent = app) entered from desktop via m/r.
  # Each is its own submap so the daemon can give it a distinct semantic border
  # colour (desktop=active/cyan, move=link/blue, resize=highlight/purple).
  modeGraph = {
    root = "app";
    modes = {
      app = { parent = null; type = "normal"; };
      desktop = { parent = "app"; type = "submap"; };
      move = { parent = "app"; type = "submap"; };
      resize = { parent = "app"; type = "submap"; };
      console = { parent = "app"; type = "passthrough"; };
    };
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

        # ── System console (fullscreen tmux overlay, available everywhere) ──
        console = { key = "F12"; action = consoleToggleAction; description = "Toggle system console"; };

        # Desktop mode is entered by CapsLock itself — the engine reads
        # `keybindings.layers.desktopToggle.entersMode` (tap = sticky, hold =
        # momentary). No keysym-bridge entry binding is needed.
      };
    };

    # Desktop mode (caps) — the WM hub. Border = active/cyan (set by daemon).
    #   arrows / hjkl = focus
    #   m = enter MOVE sub-mode (blue), r = enter RESIZE sub-mode (purple)
    #   numbers = workspace, Shift+number = send window there + follow
    # Hold CapsLock the whole time you work here; release to return to app.
    desktop = {
      enter = null;
      exit = "escape";
      bindings = {
        # ── Focus: direction (arrows + hjkl), repeats while held ──
        focusLeft = { key = "h"; action = "movefocus, l"; description = "Focus left"; repeat = true; };
        focusDown = { key = "j"; action = "movefocus, d"; description = "Focus down"; repeat = true; };
        focusUp = { key = "k"; action = "movefocus, u"; description = "Focus up"; repeat = true; };
        focusRight = { key = "l"; action = "movefocus, r"; description = "Focus right"; repeat = true; };
        focusLeftArrow = { key = "left"; action = "movefocus, l"; description = "Focus left"; repeat = true; };
        focusDownArrow = { key = "down"; action = "movefocus, d"; description = "Focus down"; repeat = true; };
        focusUpArrow = { key = "up"; action = "movefocus, u"; description = "Focus up"; repeat = true; };
        focusRightArrow = { key = "right"; action = "movefocus, r"; description = "Focus right"; repeat = true; };

        # ── Enter MOVE / RESIZE sub-modes (native submap → daemon colours them) ──
        enterMove = { key = "m"; action = "submap, move"; description = "Move-window mode"; };
        enterResize = { key = "r"; action = "submap, resize"; description = "Resize-window mode"; };

        # ── Workspaces: number = go there ──
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

        # ── Send window to workspace AND follow it: Shift + number ──
        sendToWs1 = { key = "shift + 1"; action = "movetoworkspace, 1"; description = "Send window to ws 1 + follow"; };
        sendToWs2 = { key = "shift + 2"; action = "movetoworkspace, 2"; description = "Send window to ws 2 + follow"; };
        sendToWs3 = { key = "shift + 3"; action = "movetoworkspace, 3"; description = "Send window to ws 3 + follow"; };
        sendToWs4 = { key = "shift + 4"; action = "movetoworkspace, 4"; description = "Send window to ws 4 + follow"; };
        sendToWs5 = { key = "shift + 5"; action = "movetoworkspace, 5"; description = "Send window to ws 5 + follow"; };
        sendToWs6 = { key = "shift + 6"; action = "movetoworkspace, 6"; description = "Send window to ws 6 + follow"; };
        sendToWs7 = { key = "shift + 7"; action = "movetoworkspace, 7"; description = "Send window to ws 7 + follow"; };
        sendToWs8 = { key = "shift + 8"; action = "movetoworkspace, 8"; description = "Send window to ws 8 + follow"; };
        sendToWs9 = { key = "shift + 9"; action = "movetoworkspace, 9"; description = "Send window to ws 9 + follow"; };
        sendToWs10 = { key = "shift + 0"; action = "movetoworkspace, 10"; description = "Send window to ws 10 + follow"; };

        # ── Window state ──
        # One-shot commands return to app (exitAfter); float + split stay so you
        # can keep arranging the window you just floated/split.
        closeWindow = { key = "q"; action = "killactive,"; description = "Close window"; exitAfter = true; };
        fullscreen = { key = "f"; action = "fullscreen"; description = "Fullscreen"; exitAfter = true; };
        toggleFloat = { key = "y"; action = "togglefloating,"; description = "Float (yank from tiling)"; };
        toggleSplit = { key = "tab"; action = "layoutmsg, togglesplit"; description = "Toggle split orientation"; };

        # ── Quick launches (auto-exit desktop so you can use the app) ──
        openTerminal = { key = "t"; action = "exec, $TERMINAL"; description = "Terminal"; exitAfter = true; };
        openBrowser = { key = "e"; action = "exec, $BROWSER"; description = "Browser"; exitAfter = true; };
        openLauncher = { key = "space"; action = "exec, walker -p 'Start…' -w 1000 -h 700"; description = "Launcher"; exitAfter = true; };

        # ── Notifications + lock (one-shot → return to app) ──
        dismissNotification = { key = "d"; action = "exec, makoctl dismiss"; description = "Dismiss notification"; exitAfter = true; };
        dismissAll = { key = "shift + d"; action = "exec, makoctl dismiss --all"; description = "Dismiss all"; exitAfter = true; };
        lock = { key = "x"; action = "exec, hyprlock"; description = "Lock screen"; exitAfter = true; };

        # ── Console + history ──
        console = { key = "F12"; action = consoleToggleAction; description = "System console"; };
        undoSession = { key = "u"; action = "exec, vogix session undo"; description = "Undo last window change"; };

        help = { key = "slash"; action = "exec, vogix-modes-desktop"; description = "Show keybindings"; };

        # Exits are owned by the engine: tap CapsLock toggles sticky off, releasing
        # a held CapsLock leaves a momentary mode, and Esc is the safety-net exit.
      };
    };

    # Move-window sub-mode (entered with 'm' from desktop). Border = link/blue.
    # arrows / hjkl move the active window; Esc / click-caps / release-caps exit.
    move = {
      enter = "m";
      exit = "escape";
      bindings = {
        moveLeft = { key = "h"; action = "movewindow, l"; description = "Move window left"; repeat = true; };
        moveDown = { key = "j"; action = "movewindow, d"; description = "Move window down"; repeat = true; };
        moveUp = { key = "k"; action = "movewindow, u"; description = "Move window up"; repeat = true; };
        moveRight = { key = "l"; action = "movewindow, r"; description = "Move window right"; repeat = true; };
        moveLeftArrow = { key = "left"; action = "movewindow, l"; description = "Move window left"; repeat = true; };
        moveDownArrow = { key = "down"; action = "movewindow, d"; description = "Move window down"; repeat = true; };
        moveUpArrow = { key = "up"; action = "movewindow, u"; description = "Move window up"; repeat = true; };
        moveRightArrow = { key = "right"; action = "movewindow, r"; description = "Move window right"; repeat = true; };

        toResize = { key = "r"; action = "submap, resize"; description = "Switch to resize mode"; };
        help = { key = "slash"; action = "exec, vogix-modes-move"; description = "Show keybindings"; };
      };
    };

    # Resize-window sub-mode (entered with 'r' from desktop). Border = highlight/purple.
    # arrows / hjkl resize the active window (grow toward the arrow); Esc exits.
    resize = {
      enter = "r";
      exit = "escape";
      bindings = {
        resizeLeft = { key = "h"; action = "resizeactive, -40 0"; description = "Narrower"; repeat = true; };
        resizeDown = { key = "j"; action = "resizeactive, 0 40"; description = "Taller"; repeat = true; };
        resizeUp = { key = "k"; action = "resizeactive, 0 -40"; description = "Shorter"; repeat = true; };
        resizeRight = { key = "l"; action = "resizeactive, 40 0"; description = "Wider"; repeat = true; };
        resizeLeftArrow = { key = "left"; action = "resizeactive, -40 0"; description = "Narrower"; repeat = true; };
        resizeDownArrow = { key = "down"; action = "resizeactive, 0 40"; description = "Taller"; repeat = true; };
        resizeUpArrow = { key = "up"; action = "resizeactive, 0 -40"; description = "Shorter"; repeat = true; };
        resizeRightArrow = { key = "right"; action = "resizeactive, 40 0"; description = "Wider"; repeat = true; };

        toMove = { key = "m"; action = "submap, move"; description = "Switch to move mode"; };
        help = { key = "slash"; action = "exec, vogix-modes-resize"; description = "Show keybindings"; };
      };
    };

    # Console mode: system terminal overlay (tmux)
    # Keys pass through to tmux — only F12/Escape exit the mode
    # NO catchall — unlike other modes, unbound keys go to the terminal
    console = {
      enter = null;
      # Single exec binding: toggle workspace (starts close animation), delay, then reset submap.
      # The sleep ensures the Hyprland slide-out animation completes before submap resets.
      # Without the delay, submap reset fires in the same frame and kills the animation.
      bindings = {
        exitConsole = {
          key = "F12";
          action = "exec, hyprctl dispatch togglespecialworkspace console && sleep 0.3 && hyprctl dispatch submap reset && hyprctl --batch 'keyword general:col.active_border rgb(585b70) ; keyword general:col.inactive_border rgb(313244)'";
          description = "Close console";
        };
      };
    };
  };
}

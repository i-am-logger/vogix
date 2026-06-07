# Hyprland keybinding generator
#
# Generates:
# - Normal (root/app) mode bindings (bind/binde in settings) — workspaces, media,
#   Super combos. Submap-entry binds are filtered out: the vogix input engine
#   owns mode switching and dispatches WM actions over the Hyprland IPC socket.
# - Passthrough submaps only (e.g. the console) in extraConfig — navigation
#   modes (desktop/move/resize) live in the engine, not in Hyprland submaps.
# - Mouse bindings (bindm).
#
# These plain binds also serve as a fallback: if the input engine fails to
# start, the real keyboard still reaches Hyprland and workspace/media/Super
# binds keep working. Per-mode visual feedback (border colours) is driven off
# the keypress path by the vogix daemon, not from here.
{ lib }:

let
  kbLib = import ../lib.nix { inherit lib; };
  inherit (kbLib) toHyprlandBind;

  inherit (lib)
    concatStringsSep
    mapAttrsToList
    filterAttrs
    optionalString
    ;

  # Generate bind entries for normal (root/app) mode.
  mkNormalBinds = modKey: bindings:
    let
      regular = filterAttrs (_: b: !(b.repeat or false)) bindings;
      repeating = filterAttrs (_: b: b.repeat or false) bindings;
    in
    {
      bind = mapAttrsToList
        (_name: binding:
          let hyprBind = toHyprlandBind modKey binding.key;
          in "${hyprBind}, ${binding.action}"
        )
        regular;

      binde = mapAttrsToList
        (_name: binding:
          let hyprBind = toHyprlandBind modKey binding.key;
          in "${hyprBind}, ${binding.action}"
        )
        repeating;
    };

  # Generate mouse binding entries
  mkMouseBindings = modKey: mouseBindings:
    mapAttrsToList
      (_name: binding:
        "${lib.toUpper modKey}, ${binding.button}, ${binding.action}"
      )
      mouseBindings;

  # Generate passthrough submap (keys pass to underlying app, only explicit bindings work)
  mkPassthroughSubmap = modKey: name: mode:
    let
      bindings = mode.bindings or { };
      bindLines = concatStringsSep "\n" (
        mapAttrsToList
          (_: b:
            let hyprBind = toHyprlandBind modKey (b.key or "");
            in "bind = ${hyprBind}, ${b.action or ""}"
          )
          bindings
      );
    in
    ''
      submap = ${name}
      ${bindLines}
      submap = reset
    '';

  # Main generator — driven by modeGraph
  generate = cfg:
    let
      inherit (cfg) modKey;
      modes = cfg.modes or { };
      modeGraph = cfg.modeGraph or { root = "app"; modes = { app = { parent = null; type = "normal"; }; }; };

      rootMode = modeGraph.root;
      graphModes = modeGraph.modes;

      # The vogix input engine owns the mode statechart: it drives navigation by
      # dispatching concrete actions over Hyprland's IPC socket and never asks
      # Hyprland to switch submaps. So the native navigation submaps and their
      # `submap, X` entry binds are dropped here — only passthrough submaps (the
      # console) are emitted, entered by their own exec. The plain workspace /
      # media / Super dispatches are KEPT as a fallback: if the engine fails to
      # start (it caps its own restart loop), the real keyboard still reaches
      # Hyprland and those binds keep working.
      isSubmapAction = b: lib.hasPrefix "submap" (b.action or "");

      # Root mode → settings.bind/binde (submap-entry binds are engine-owned).
      normalMode = modes.${rootMode} or { bindings = { }; };
      normalBindings = filterAttrs (_: b: !(isSubmapAction b)) (normalMode.bindings or { });
      normalBinds = mkNormalBinds modKey normalBindings;

      # Only passthrough submaps (the console) survive; navigation submaps are
      # engine-owned and omitted.
      submapList = mapAttrsToList
        (name: graphDef:
          let
            mode = modes.${name} or null;
            modeType = graphDef.type or "submap";
          in
          if mode != null && name != rootMode && modeType == "passthrough"
          then mkPassthroughSubmap modKey name mode
          else ""
        )
        graphModes;

      submapConfigs = concatStringsSep "\n\n" (
        builtins.filter (s: s != "") submapList
      );

      mouseBinds = mkMouseBindings modKey (cfg.mouse or { });

      inputCfg = cfg.input or { };
      touchpadCfg = cfg.touchpad or { };
      layoutsCfg = cfg.layouts or { };
      miscCfg = cfg.misc or { };
    in
    {
      settings = {
        "$mainMod" = lib.toUpper modKey;
        inherit (normalBinds) bind binde;
        bindm = mouseBinds;
        binds = {
          workspace_back_and_forth = false;
          allow_workspace_cycles = false;
        };

        # Input settings
        input = {
          repeat_delay = inputCfg.repeatDelay or 200;
          sensitivity = inputCfg.sensitivity or 0.0;
          left_handed = inputCfg.leftHanded or false;
          natural_scroll = if inputCfg.naturalScroll or true then "yes" else "no";
          float_switch_override_focus = inputCfg.floatSwitchOverrideFocus or 2;
          numlock_by_default = if inputCfg.numlockByDefault or false then "on" else "off";

          touchpad = {
            natural_scroll = if touchpadCfg.naturalScroll or true then 1 else 0;
            disable_while_typing = touchpadCfg.disableWhileTyping or true;
            scroll_factor = touchpadCfg.scrollFactor or 0.3;
          };
        };

        # Layout
        general.layout = cfg.layout or "dwindle";
        # No `pseudotile` — Hyprland removed the dwindle:pseudotile option (≥0.55).
        dwindle = layoutsCfg.dwindle or { preserve_split = true; force_split = 2; };
        master = layoutsCfg.master or { new_status = "slave"; new_on_top = true; };

        # Misc
        misc = {
          font_family = miscCfg.fontFamily or "Fira Code Nerd Font";
          disable_hyprland_logo = miscCfg.disableLogo or true;
          disable_autoreload = miscCfg.disableAutoreload or false;
          always_follow_on_dnd = miscCfg.alwaysFollowOnDnd or true;
          layers_hog_keyboard_focus = miscCfg.layersHogKeyboardFocus or true;
          animate_manual_resizes = miscCfg.animateManualResizes or true;
          enable_swallow = miscCfg.enableSwallow or false;
          focus_on_activate = miscCfg.focusOnActivate or true;
        };

        # Gestures
        gestures = cfg.gestures or { };

        # Console window rules (enabled when console mode exists in mode graph)
        windowrule = lib.optionals (graphModes ? console) [
          "match:class ^(vogix-console)$, workspace special:console"
          "match:class ^(vogix-console)$, float true"
          "match:class ^(vogix-console)$, size 90% 75%"
          "match:class ^(vogix-console)$, center true"
        ];

        # Console workspace
        workspace = lib.optionals (graphModes ? console) [
          "special:console, persistent:true, gapsout:0, gapsin:0, shadow:false, on-created-empty:wezterm start --class vogix-console -- tmux new-session -A -s console"
        ];
      };

      extraConfig = optionalString (submapConfigs != "") submapConfigs;
    };

in
{
  inherit generate;
}

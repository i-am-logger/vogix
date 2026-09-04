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
  inherit (kbLib) toHyprlandBind toLuaKeys;

  luaLib = import ../../lib/hypr-lua.nix { inherit lib; };
  inherit (luaLib) actionToLua;
  inherit (lib.generators) mkLuaInline;

  inherit (lib)
    concatStringsSep
    mapAttrsToList
    filterAttrs
    optionalString
    ;

  # Shared selection logic for both renderers: which bindings land in the
  # root config and which modes become passthrough submaps. The vogix input
  # engine owns the mode statechart, so submap-ENTRY binds are dropped from
  # the root and only passthrough submaps (the console) are emitted at all —
  # see the comments in `generate` below.
  analyze = cfg:
    let
      modes = cfg.modes or { };
      modeGraph = cfg.modeGraph or { root = "app"; modes = { app = { parent = null; type = "normal"; }; }; };
      rootMode = modeGraph.root;
      graphModes = modeGraph.modes;
      isSubmapAction = b: lib.hasPrefix "submap" (b.action or "");
      normalMode = modes.${rootMode} or { bindings = { }; };
    in
    {
      inherit rootMode graphModes;
      normalBindings = filterAttrs (_: b: !(isSubmapAction b)) (normalMode.bindings or { });
      passthroughModes = filterAttrs (_: m: m != null) (
        lib.mapAttrs
          (name: graphDef:
            let mode = modes.${name} or null;
            in
            if mode != null && name != rootMode && (graphDef.type or "submap") == "passthrough"
            then mode
            else null
          )
          graphModes
      );
    };

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

      # The vogix input engine owns the mode statechart: it drives navigation by
      # dispatching concrete actions over Hyprland's IPC socket and never asks
      # Hyprland to switch submaps. So the native navigation submaps and their
      # `submap, X` entry binds are dropped here — only passthrough submaps (the
      # console) are emitted, entered by their own exec. The plain workspace /
      # media / Super dispatches are KEPT as a fallback: if the engine fails to
      # start (it caps its own restart loop), the real keyboard still reaches
      # Hyprland and those binds keep working.
      inherit (analyze cfg) graphModes normalBindings passthroughModes;

      # Root mode → settings.bind/binde (submap-entry binds are engine-owned).
      normalBinds = mkNormalBinds modKey normalBindings;

      submapConfigs = concatStringsSep "\n\n" (
        mapAttrsToList (name: mkPassthroughSubmap modKey name) passthroughModes
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
          kb_layout = inputCfg.kbLayout or "us";
          kb_options = inputCfg.kbOptions or "";
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

  # One overlay binding → the HM Lua bind element
  # ({ _args = [ keys dispatcher opts? ]; } → `hl.bind(keys, hl.dsp.*, opts)`).
  # `binde` disappears as a separate list: repeat is the `repeating` opt.
  mkLuaBind = modKey: binding:
    let
      opts =
        lib.optionalAttrs (binding.repeat or false) { repeating = true; }
        // lib.optionalAttrs (binding.description or "" != "") { inherit (binding) description; };
    in
    {
      _args = [
        (toLuaKeys modKey (binding.key or ""))
        (mkLuaInline (actionToLua (binding.action or "")))
      ] ++ lib.optional (opts != { }) opts;
    };

  # Mouse drag/resize: the legacy `bindm` flag has no Lua form — the
  # `hl.dsp.window.drag()`/`.resize()` dispatchers carry the press/release
  # lifecycle themselves.
  mkLuaMouseBinds = modKey: mouseBindings:
    mapAttrsToList
      (_name: binding:
        let
          dsp =
            if binding.action == "movewindow" then "hl.dsp.window.drag()"
            else if binding.action == "resizewindow" then "hl.dsp.window.resize()"
            else throw "vogix: mouse bind action '${binding.action}' has no Lua drag/resize form";
        in
        { _args = [ "${lib.toUpper modKey} + ${binding.button}" (mkLuaInline dsp) ]; }
      )
      mouseBindings;

  # Lua-engine renderer — the same selection logic as `generate`, projected
  # onto the HM Lua shapes (`settings.<name>` → one `hl.<name>(...)` per list
  # element; `settings.config` → one `hl.config({...})`; `submaps.<name>` →
  # `hl.define_submap`). Returns { settings, submaps }.
  generateLua = cfg:
    let
      inherit (cfg) modKey;
      inherit (analyze cfg) graphModes normalBindings passthroughModes;

      inputCfg = cfg.input or { };
      touchpadCfg = cfg.touchpad or { };
      layoutsCfg = cfg.layouts or { };
      miscCfg = cfg.misc or { };
      gesturesCfg = cfg.gestures or { };
    in
    {
      settings = {
        bind =
          mapAttrsToList (_: mkLuaBind modKey) normalBindings
          ++ mkLuaMouseBinds modKey (cfg.mouse or { });

        # Everything that was a plain hyprlang section is one hl.config call.
        # Booleans are real booleans: the Lua engine hard-rejects the
        # hyprlang "yes"/"no"/"on"/"off" strings.
        config = {
          binds = {
            workspace_back_and_forth = false;
            allow_workspace_cycles = false;
          };

          input = {
            kb_layout = inputCfg.kbLayout or "us";
            kb_options = inputCfg.kbOptions or "";
            repeat_delay = inputCfg.repeatDelay or 200;
            sensitivity = inputCfg.sensitivity or 0.0;
            left_handed = inputCfg.leftHanded or false;
            natural_scroll = inputCfg.naturalScroll or true;
            float_switch_override_focus = inputCfg.floatSwitchOverrideFocus or 2;
            numlock_by_default = inputCfg.numlockByDefault or false;

            touchpad = {
              natural_scroll = touchpadCfg.naturalScroll or true;
              disable_while_typing = touchpadCfg.disableWhileTyping or true;
              scroll_factor = touchpadCfg.scrollFactor or 0.3;
            };
          };

          general.layout = cfg.layout or "dwindle";
          # No `pseudotile` — Hyprland removed the dwindle:pseudotile option (≥0.55).
          dwindle = layoutsCfg.dwindle or { preserve_split = true; force_split = 2; };
          master = layoutsCfg.master or { new_status = "slave"; new_on_top = true; };

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
        } // lib.optionalAttrs (gesturesCfg != { }) { gestures = gesturesCfg; };

        # Console rules: the four hyprlang effect lines collapse to one named
        # table (named rules deduplicate across reloads and get a handle).
        window_rule = lib.optionals (graphModes ? console) [
          {
            name = "vogix-console";
            match.class = "^(vogix-console)$";
            workspace = "special:console";
            float = true;
            size = "90% 75%";
            center = true;
          }
        ];

        # `shadow:false` inverts to `no_shadow`; the on-created-empty command
        # needs no `[workspace …]` prefix (the Lua path passes the rule's
        # workspace at spawn time).
        workspace_rule = lib.optionals (graphModes ? console) [
          {
            workspace = "special:console";
            persistent = true;
            gaps_out = 0;
            gaps_in = 0;
            no_shadow = true;
            on_created_empty = "wezterm start --class vogix-console -- tmux new-session -A -s console";
          }
        ];
      };

      # Passthrough submaps (the console). No `onDispatch`: a passthrough
      # submap persists until one of its own binds resets it. A submap with no
      # bindings is omitted entirely — the `submap` dispatcher can switch to an
      # undeclared submap, and HM drops empty Lua submaps anyway.
      submaps = lib.mapAttrs
        (_name: mode: {
          settings.bind = mapAttrsToList (_: mkLuaBind modKey) (mode.bindings or { });
        })
        (filterAttrs (_: mode: (mode.bindings or { }) != { }) passthroughModes);
    };

in
{
  inherit generate generateLua;
}

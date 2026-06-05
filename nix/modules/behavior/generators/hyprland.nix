# Hyprland keybinding generator — Modal design
#
# Generates:
# - Normal mode bindings (bind/binde in settings) — workspaces, media, Super+non-letter
# - Desktop/theme submaps (extraConfig) — single-key modal actions
# - Mouse bindings (bindm)
#
# Visual feedback: ALL borders change color per mode (like vim's mode indicator)
# Border colors derived from vogix semantic theme colors via modeColors config.
# Waybar's hyprland/submap module also reads the current submap automatically.
#
# Submap flow (flat — every submap parented to app, single-Esc exit):
#   reset (app mode) ↔ desktop
#                    ↔ theme
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

  # ── Submap transitions are NATIVE (synchronous), never exec ──
  #
  # CRITICAL: a "submap, X" action MUST be emitted verbatim so Hyprland's native
  # submap dispatcher runs it synchronously, switching the submap BEFORE the next
  # key event is processed. This is what makes momentary mode work: caps held →
  # kanata emits Scroll_Lock then (≈1ms later) the arrow; Hyprland must enter the
  # submap on Scroll_Lock before handling the arrow.
  #
  # A previous version wrapped transitions into `exec, hyprctl … dispatch submap`
  # to also set per-mode border colours. That exec is ASYNC (~6–7ms process
  # spawn + IPC) — the arrow arrived first and leaked to the app, so momentary
  # mode did nothing. Border colours belong OFF the keypress path (the vogix
  # daemon already watches submap changes — that is their home), never here.
  #
  # So actions pass through unchanged: "submap, X" stays native; exec actions
  # (launches, console toggle) stay exec.

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

  # Generate a submap block. `holdExitKey` is the Hyprland keysym kanata taps
  # when CapsLock-hold is RELEASED; a press-`bind` on it exits the submap.
  mkSubmapBlock = holdExitKey: modKey: name: mode: parentSubmap:
    let
      bindings = mode.bindings or { };
      exitKey = mode.exit or "escape";

      regularBindings = filterAttrs (_: b: !(b.repeat or false)) bindings;
      repeatingBindings = filterAttrs (_: b: b.repeat or false) bindings;

      # "killactive," → "killactive"; "movetoworkspace, 3" → "movetoworkspace 3"
      toDispatch = a: lib.removeSuffix "," (builtins.replaceStrings [ ", " ] [ " " ] a);

      # Resolve a binding's action. exitAfter returns to app after the action
      # (one-shot commands). These are NOT on the momentary critical path — no
      # following key depends on the timing — so an async exec reset is fine:
      #   exec actions  → reset first, then run the command
      #   non-exec      → dispatch the action, then reset
      # Everything else passes through unchanged (native "submap, X" stays
      # synchronous; this is what makes momentary mode work).
      resolveAction = binding:
        if (binding.exitAfter or false) then
          (if lib.hasPrefix "exec, " binding.action
          then "exec, hyprctl dispatch submap reset ; ${lib.removePrefix "exec, " binding.action}"
          else "exec, hyprctl dispatch ${toDispatch binding.action} ; hyprctl dispatch submap reset")
        else binding.action;

      regularLines = concatStringsSep "\n" (
        mapAttrsToList
          (_: binding:
            let hyprBind = toHyprlandBind modKey binding.key;
            in "bind = ${hyprBind}, ${resolveAction binding}"
          )
          regularBindings
      );

      repeatingLines = concatStringsSep "\n" (
        mapAttrsToList
          (_: binding:
            let hyprBind = toHyprlandBind modKey binding.key;
            in "binde = ${hyprBind}, ${resolveAction binding}"
          )
          repeatingBindings
      );

      # Native submap exits — synchronous, so a key after the exit lands in app.
      exitAction = "submap, ${parentSubmap}";
    in
    ''
      submap = ${name}
      ${regularLines}
      ${optionalString (repeatingLines != "") repeatingLines}
      bind = , ${exitKey}, ${exitAction}
      bind = , ${holdExitKey}, submap, reset
      bind = , catchall, exec,
      submap = reset
    '';

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

      # The Hyprland keysym that EXITS a momentary submap. kanata taps this key
      # (holdReleaseAction) when CapsLock-hold is released, and Hyprland exits via
      # a normal press-`bind`. We deliberately do NOT use `bindr` on the hold key:
      # Hyprland's release-binds don't fire when the press entered the submap, so
      # the mode would stick. A separate exit keypress + press-bind is reliable.
      kanataToHyprKeysym = k:
        if k == "slck" then "Scroll_Lock"
        else if k == null then "F22"
        else lib.toUpper k; # f22 → F22
      holdExitKey = kanataToHyprKeysym
        (cfg.layers.desktopToggle.holdReleaseAction or null);

      # Under the vogix input engine the engine owns the mode statechart and
      # drives navigation by dispatching concrete actions over Hyprland's IPC
      # socket — it never asks Hyprland to switch submaps. So the native submap
      # blocks and their `submap, X` entry binds (F23/F24/F3) are orphaned:
      # nothing emits their entry keysyms (kanata is disabled under this engine),
      # and a submap that Hyprland somehow *did* enter would trap the re-emitted
      # keystream in a catchall — the "stuck in a mode" bug the engine exists to
      # make unrepresentable. Drop them under this engine — this implements the
      # promise already in the `inputEngine` option doc ("the Hyprland submap
      # binds are omitted entirely under this engine"). The plain workspace /
      # media / Super dispatches are KEPT as a fallback: if the engine fails to
      # start (it caps its own restart loop), the real keyboard still reaches
      # Hyprland and those binds keep working.
      engineOwnsModes = (cfg.inputEngine or "kanata") == "vogix";
      isSubmapAction = b: lib.hasPrefix "submap" (b.action or "");

      # Root mode → settings.bind/binde
      normalMode = modes.${rootMode} or { bindings = { }; };
      normalBindings =
        if engineOwnsModes
        then filterAttrs (_: b: !(isSubmapAction b)) (normalMode.bindings or { })
        else normalMode.bindings or { };
      normalBinds = mkNormalBinds modKey normalBindings;

      # Generate submaps from mode graph — all non-root modes. Under the vogix
      # engine only passthrough submaps (the console) survive; the navigation
      # submaps are engine-owned (see `engineOwnsModes` above).
      submapList = mapAttrsToList
        (name: graphDef:
          let
            mode = modes.${name} or null;
            modeType = graphDef.type or "submap";
            parentName = graphDef.parent or rootMode;
            # In Hyprland, "reset" means the default (no submap). Root mode = "reset".
            parentSubmap = if parentName == rootMode then "reset" else parentName;
          in
          if name == rootMode || mode == null then ""
          else if engineOwnsModes && modeType != "passthrough" then ""
          else if modeType == "passthrough" then
            mkPassthroughSubmap modKey name mode
          else
            mkSubmapBlock holdExitKey modKey name mode parentSubmap
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

# Hyprland keybinding generator — Modal design
#
# Generates:
# - Normal mode bindings (bind/binde in settings) — workspaces, media, Super+non-letter
# - Desktop/arrange/theme submaps (extraConfig) — single-key modal actions
# - Mouse bindings (bindm)
#
# Visual feedback: ALL borders change color per mode (like vim's mode indicator)
# Border colors derived from vogix semantic theme colors via modeColors config.
# Waybar's hyprland/submap module also reads the current submap automatically.
#
# Submap flow:
#   reset (app mode) ↔ desktop ↔ arrange
#                              ↔ theme
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

  # Fallback border colors if modeColors not provided
  fallbackBorders = {
    app = { active = "rgb(585b70)"; inactive = "rgb(313244)"; };
    desktop = { active = "rgb(89b4fa)"; inactive = "rgb(313244)"; };
    arrange = { active = "rgb(f9e2af)"; inactive = "rgb(313244)"; };
    theme = { active = "rgb(a6e3a1)"; inactive = "rgb(313244)"; };
    console = { active = "rgb(cba6f7)"; inactive = "rgb(3b2d4f)"; }; # special — system console
  };

  # Resolve colors for a mode (from modeColors config or fallback)
  getModeColors = modeColors: name:
    let colorKey = if name == "reset" then "app" else name;
    in modeColors.${colorKey} or fallbackBorders.${colorKey} or fallbackBorders.app;

  # Wrap a "submap, X" action with border color change
  wrapSubmapAction = modeColors: action:
    let
      parts = lib.splitString ", " action;
      isSubmap = builtins.length parts == 2 && builtins.head parts == "submap";
      isConsole = builtins.length parts == 2 && builtins.head parts == "togglespecialworkspace";
      target = if isSubmap then lib.last parts else null;
      colors = if isSubmap then getModeColors modeColors target else null;
      consoleColors = getModeColors modeColors "console";
    in
    if isSubmap then
      "exec, hyprctl --batch 'keyword general:col.active_border ${colors.active} ; keyword general:col.inactive_border ${colors.inactive} ; dispatch submap ${target}'"
    else if isConsole then
      let
        wsName = lib.last parts;
        appColors = getModeColors modeColors "app";
      in
      "exec, hyprctl dispatch togglespecialworkspace ${wsName} && (hyprctl workspaces -j | grep -q '\"special:${wsName}\"' && hyprctl --batch 'keyword general:col.active_border ${consoleColors.active} ; keyword general:col.inactive_border ${consoleColors.inactive}' || hyprctl --batch 'keyword general:col.active_border ${appColors.active} ; keyword general:col.inactive_border ${appColors.inactive}')"
    else
      action;

  # Generate bind entries for normal mode
  mkNormalBinds = modeColors: modKey: bindings:
    let
      regular = filterAttrs (_: b: !(b.repeat or false)) bindings;
      repeating = filterAttrs (_: b: b.repeat or false) bindings;
    in
    {
      bind = mapAttrsToList
        (_name: binding:
          let
            hyprBind = toHyprlandBind modKey binding.key;
            action = wrapSubmapAction modeColors binding.action;
          in
          "${hyprBind}, ${action}"
        )
        regular;

      binde = mapAttrsToList
        (_name: binding:
          let hyprBind = toHyprlandBind modKey binding.key;
          in "${hyprBind}, ${binding.action}"
        )
        repeating;
    };

  # Generate a submap block
  mkSubmapBlock = modeColors: modKey: name: mode: parentSubmap:
    let
      bindings = mode.bindings or { };
      exitKey = mode.exit or "escape";

      regularBindings = filterAttrs (_: b: !(b.repeat or false)) bindings;
      repeatingBindings = filterAttrs (_: b: b.repeat or false) bindings;

      regularLines = concatStringsSep "\n" (
        mapAttrsToList
          (_: binding:
            let
              hyprBind = toHyprlandBind modKey binding.key;
              action = wrapSubmapAction modeColors binding.action;
            in
            "bind = ${hyprBind}, ${action}"
          )
          regularBindings
      );

      repeatingLines = concatStringsSep "\n" (
        mapAttrsToList
          (_: binding:
            let hyprBind = toHyprlandBind modKey binding.key;
            in "binde = ${hyprBind}, ${binding.action}"
          )
          repeatingBindings
      );

      exitAction = wrapSubmapAction modeColors "submap, ${parentSubmap}";
    in
    ''
      submap = ${name}
      ${regularLines}
      ${optionalString (repeatingLines != "") repeatingLines}
      bind = , ${exitKey}, ${exitAction}
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

  # Main generator
  generate = cfg:
    let
      inherit (cfg) modKey;
      modes = cfg.modes or { };
      modeColors = cfg.modeColors or { };

      # Normal mode → settings.bind/binde
      normalMode = modes.normal or { bindings = { }; };
      normalBinds = mkNormalBinds modeColors modKey (normalMode.bindings or { });

      # Desktop mode → submap (Escape returns to reset/app mode)
      desktopMode = modes.desktop or { bindings = { }; };
      desktopSubmap = mkSubmapBlock modeColors modKey "desktop" desktopMode "reset";

      # Arrange mode → submap (Escape returns to desktop mode)
      arrangeMode = modes.arrange or null;
      arrangeSubmap = optionalString (arrangeMode != null)
        (mkSubmapBlock modeColors modKey "arrange" arrangeMode "desktop");

      # Theme mode → submap (Escape returns to desktop mode)
      themeMode = modes.theme or null;
      themeSubmap = optionalString (themeMode != null)
        (mkSubmapBlock modeColors modKey "theme" themeMode "desktop");

      # Console mode → passthrough submap (only F12 exits, no catchall — keys go to terminal)
      # Bindings are emitted raw (no wrapSubmapAction) so native dispatchers work for animation
      consoleMode = modes.console or null;
      consoleSubmap = optionalString (consoleMode != null)
        (
          let
            bindings = consoleMode.bindings or { };
            bindLines = concatStringsSep "\n" (
              mapAttrsToList
                (_: b:
                  let
                    hyprBind = toHyprlandBind modKey (b.key or "");
                  in
                  "bind = ${hyprBind}, ${b.action or ""}"
                )
                bindings
            );
          in
          ''
            submap = console
            ${bindLines}
            submap = reset
          ''
        );

      submapConfigs = concatStringsSep "\n\n" (
        builtins.filter (s: s != "") [
          desktopSubmap
          arrangeSubmap
          themeSubmap
          consoleSubmap
        ]
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
        dwindle = layoutsCfg.dwindle or { pseudotile = true; preserve_split = true; force_split = 2; };
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

        # Console window rules
        windowrule = lib.optionals (consoleMode != null) [
          "match:class ^(vogix-console)$, workspace special:console"
          "match:class ^(vogix-console)$, float true"
          "match:class ^(vogix-console)$, size 90% 75%"
          "match:class ^(vogix-console)$, center true"
        ];

        # Console workspace
        workspace = lib.optionals (consoleMode != null) [
          "special:console, persistent:true, gapsout:0, gapsin:0, shadow:false, on-created-empty:wezterm start --class vogix-console -- tmux new-session -A -s console"
        ];
      };

      extraConfig = optionalString (submapConfigs != "") submapConfigs;
    };

in
{
  inherit generate;
}

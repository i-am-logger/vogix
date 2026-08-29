# Legacy Hyprland → Lua config projection helpers.
#
# The Nix mirror of `src/input/hypr_lua.rs`: the same legacy dispatcher →
# `hl.dsp.*` translation table, for the config GENERATION path (the runtime
# IPC translation lives in Rust). The mapping was read out of the Hyprland
# 0.56.2 sources and verified against nested instances; the wire contract is
# documented in `docs/hyprland-lua-ipc.md`. Any dispatcher without a Lua
# translation throws at evaluation time — emitting the legacy string into a
# Lua config would be a guaranteed syntax error, so failing the build is the
# honest behaviour.
#
# Consumed by `behavior/generators/hyprland.nix` (generateLua), by
# `appearance/default.nix` (mkHyprlandLuaConfig) and by mynixos's Hyprland
# module for its own legacy bind strings.
{ lib }:

let
  inherit (lib)
    concatStringsSep
    hasPrefix
    hasSuffix
    optionalString
    removePrefix
    removeSuffix
    replaceStrings
    splitString
    trim
    ;

  # Quote a Nix string as a Lua short string literal — `lua_str` in
  # src/input/hypr_lua.rs. The Rust side additionally escapes raw C0 bytes;
  # strings arriving here come from Nix module options, where only \n, \r and
  # \t can occur, so the four escapes below are the complete set.
  luaStr = s: "\"${replaceStrings [ "\\" "\"" "\n" "\r" "\t" ] [ "\\\\" "\\\"" "\\n" "\\r" "\\t" ] s}\"";

  # Split a legacy action string "dispatcher, args" at the first comma.
  splitAction = action:
    let
      a = trim action;
      m = builtins.match "([^,]*),(.*)" a;
    in
    if m == null
    then { disp = a; args = ""; }
    else { disp = trim (builtins.elemAt m 0); args = trim (builtins.elemAt m 1); };

  # Legacy `resizeactive`/`moveactive` args: two plain pixel integers. The
  # legacy percentage/`exact` grammars have no Lua form — refuse rather than
  # silently mistranslate.
  parseTwoInts = disp: args:
    let m = builtins.match "(-?[0-9]+)[[:space:]]+(-?[0-9]+)" args;
    in
    if m == null
    then
      throw
        "vogix: ${disp} takes two plain pixel integers under the Lua engine (legacy percentages/`exact` have no Lua form), got '${args}'"
    else { dx = builtins.elemAt m 0; dy = builtins.elemAt m 1; };

  # `movetoworkspace[silent] <ws>[,<window-selector>]`. Silence is spelled
  # `follow = false`; there is no `silent` field.
  moveToWorkspace = args: silent:
    let
      m = builtins.match "([^,]*),(.*)" args;
      ws = if m == null then args else trim (builtins.elemAt m 0);
      window = if m == null then null else trim (builtins.elemAt m 1);
      fields = "workspace = ${luaStr ws}"
        + optionalString (window != null) ", window = ${luaStr window}"
        + optionalString silent ", follow = false";
    in
    "hl.dsp.window.move({ ${fields} })";

  # `movewindow <dir>` or `movewindow mon:<sel>[ silent]`. The Lua monitor
  # selector grammar has no `mon:` prefix — it must be stripped here.
  moveWindow = args: groupAware:
    let
      group = optionalString groupAware ", group_aware = true";
    in
    if hasPrefix "mon:" args then
      let
        rest = removePrefix "mon:" args;
        silent = hasSuffix " silent" rest;
        sel = trim (if silent then removeSuffix " silent" rest else rest);
        follow = optionalString silent ", follow = false";
      in
      "hl.dsp.window.move({ monitor = ${luaStr sel}${follow}${group} })"
    else if args == "" then
    # A bare `hl.dsp.window.move()` would start an interactive mouse drag
    # (legacy `mouse movewindow`), which is never what a keybind action
    # means — refuse instead.
      throw "vogix: movewindow needs a direction or mon:<selector>"
    else
      "hl.dsp.window.move({ direction = ${luaStr args}${group} })";

  # Translate a legacy schema action ("dispatcher, args") into the Lua
  # dispatcher expression to place in `hl.bind(...)`. Attribute lookup is
  # lazy, so only the selected arm evaluates (and only it can throw).
  actionToLua = action:
    let
      inherit (splitAction action) disp args;
      table = {
        exec = "hl.dsp.exec_cmd(${luaStr args})";
        execr = "hl.dsp.exec_raw(${luaStr args})";
        workspace = "hl.dsp.focus({ workspace = ${luaStr args} })";
        focusworkspaceoncurrentmonitor = "hl.dsp.focus({ workspace = ${luaStr args}, on_current_monitor = true })";
        movetoworkspace = moveToWorkspace args false;
        movetoworkspacesilent = moveToWorkspace args true;
        movefocus = "hl.dsp.focus({ direction = ${luaStr args} })";
        focusmonitor = "hl.dsp.focus({ monitor = ${luaStr args} })";
        focuswindow = "hl.dsp.focus({ window = ${luaStr args} })";
        swapwindow = "hl.dsp.window.swap({ direction = ${luaStr args} })";
        swapnext = "hl.dsp.window.swap({ next = true })";
        movewindow = moveWindow args false;
        movewindoworgroup = moveWindow args true;
        moveactive =
          let p = parseTwoInts "moveactive" args;
          in "hl.dsp.window.move({ x = ${p.dx}, y = ${p.dy}, relative = true })";
        resizeactive =
          let p = parseTwoInts "resizeactive" args;
          in "hl.dsp.window.resize({ x = ${p.dx}, y = ${p.dy}, relative = true })";
        # Bare-scalar dispatchers — a table argument would error (submap,
        # layout) or silently degrade to the unnamed special workspace
        # (toggle_special), so these never go through table building.
        submap = "hl.dsp.submap(${luaStr args})";
        layoutmsg = "hl.dsp.layout(${luaStr args})";
        togglespecialworkspace =
          if args == ""
          then "hl.dsp.workspace.toggle_special()"
          else "hl.dsp.workspace.toggle_special(${luaStr args})";
        killactive = "hl.dsp.window.close()";
        forcekillactive = "hl.dsp.window.kill()";
        closewindow = "hl.dsp.window.close({ window = ${luaStr args} })";
        togglefloating = "hl.dsp.window.float()";
        setfloating = "hl.dsp.window.float({ action = \"enable\" })";
        settiled = "hl.dsp.window.float({ action = \"disable\" })";
        # Legacy: 1 = maximize, any other number = fullscreen. The Lua factory
        # only accepts "maximized"/"fullscreen", so normalise the number.
        fullscreen =
          if args == "" then "hl.dsp.window.fullscreen()"
          else if args == "1" then "hl.dsp.window.fullscreen({ mode = \"maximized\" })"
          else "hl.dsp.window.fullscreen({ mode = \"fullscreen\" })";
        pin = "hl.dsp.window.pin()";
        pseudo = "hl.dsp.window.pseudo()";
        centerwindow = "hl.dsp.window.center()";
        cyclenext = "hl.dsp.window.cycle_next()";
        bringactivetotop = "hl.dsp.window.bring_to_top()";
        togglegroup = "hl.dsp.group.toggle()";
        # Legacy treats anything other than b/prev as forward.
        changegroupactive =
          if args == "b" || args == "prev"
          then "hl.dsp.group.prev()"
          else "hl.dsp.group.next()";
        movegroupwindow =
          if args == "b"
          then "hl.dsp.group.move_window({ forward = false })"
          else "hl.dsp.group.move_window()";
        # `dpms <on|off>`: the Lua factory takes the bare mode (probed live);
        # per-monitor dpms has no verified Lua form.
        dpms =
          if args == "on" || args == "off"
          then "hl.dsp.dpms(${luaStr args})"
          else throw "vogix: dpms takes 'on' or 'off' under the Lua engine, got '${args}'";
        exit = "hl.dsp.exit()";
      };
    in
      table.${disp} or (throw "vogix: no Lua translation for dispatcher '${disp}' (action '${action}'); the legacy string cannot be placed in a Lua-engine Hyprland config");

  # Legacy hyprlang bind key prefix ("$mainMod SHIFT", "Space") → the Lua
  # keys-string ("SUPER + SHIFT + Space"). Lua's parseKeyString splits on `+`
  # with uppercase-exact modifier names; keysyms stay case-insensitive.
  legacyKeysToLua = modsStr: key:
    let
      resolved = replaceStrings [ "$mainMod" ] [ "SUPER" ] modsStr;
      mods = builtins.filter (m: m != "") (splitString " " (trim resolved));
    in
    concatStringsSep " + " (mods ++ [ (trim key) ]);

  # A whole legacy bind line "MODS, key, dispatcher[, args]" → the HM Lua
  # bind element ({ _args = [ keys dispatcher opts? ]; }). `opts` is the
  # hl.bind opts table ({ } for none, { repeating = true; } for binde).
  legacyBindToLua = opts: line:
    let
      m = builtins.match "([^,]*), *([^,]*), *(.*)" line;
    in
    if m == null
    then throw "vogix: cannot parse legacy bind line '${line}'"
    else {
      _args = [
        (legacyKeysToLua (builtins.elemAt m 0) (builtins.elemAt m 1))
        (lib.generators.mkLuaInline (actionToLua (builtins.elemAt m 2)))
      ] ++ lib.optional (opts != { }) opts;
    };

  # Legacy `bezier = "NAME, X0, Y0, X1, Y1"` → the hl.curve element
  # ({ _args = [ name { type points } ]; }). The four floats become
  # `points = {{X0,Y0},{X1,Y1}}`, field for field.
  parseBezier = s:
    let
      parts = map trim (splitString "," s);
      num = i: builtins.fromJSON (builtins.elemAt parts i);
    in
    if builtins.length parts != 5
    then throw "vogix: bezier '${s}' must be 'name, x0, y0, x1, y1'"
    else {
      _args = [
        (builtins.head parts)
        {
          type = "bezier";
          points = [ [ (num 1) (num 2) ] [ (num 3) (num 4) ] ];
        }
      ];
    };

  # Legacy animation rule "LEAF, ENABLED, SPEED, BEZIER[, STYLE]" → the
  # hl.animation table. `speed` keeps the legacy unit (both handlers feed the
  # identical setConfigForNode call).
  parseAnimationRule = s:
    let
      parts = map trim (splitString "," s);
      len = builtins.length parts;
    in
    if len < 4
    then throw "vogix: animation rule '${s}' must be 'leaf, enabled, speed, bezier[, style]'"
    else
      {
        leaf = builtins.elemAt parts 0;
        enabled = builtins.elemAt parts 1 != "0";
        speed = builtins.fromJSON (builtins.elemAt parts 2);
        bezier = builtins.elemAt parts 3;
      } // lib.optionalAttrs (len > 4) { style = builtins.elemAt parts 4; };

in
{
  inherit
    actionToLua
    legacyBindToLua
    legacyKeysToLua
    luaStr
    parseAnimationRule
    parseBezier
    ;
}

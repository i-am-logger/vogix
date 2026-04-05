# Kanata configuration generator
#
# Transforms vogix keybinding layers and universal shortcuts into kanata config.
# Kanata operates at the evdev level — remappings work in every app.
#
# Generates:
# 1. Nav layer (CapsLock hold = hjkl arrows)
# 2. CapsLock tap = F13 (Hyprland catches for desktop mode toggle)
# 3. Super+letter → Ctrl+letter (macOS Command behavior)
#    ONLY for letters — Super+number and Super+mouse pass through to Hyprland
{ lib }:

let
  kbLib = import ../lib.nix { inherit lib; };
  inherit (kbLib) toKanataKey;

  inherit (lib)
    concatStringsSep
    mapAttrsToList
    ;

  # Collect all source keys that need to be intercepted for layers
  collectLayerSourceKeys = layers:
    let
      layerKeys = lib.concatMap
        (layer:
          let
            hold = layer.hold or null;
            toggle = layer.toggle or null;
            holdKeys = lib.optional (hold != null) hold;
            toggleKeys = lib.optional (toggle != null) toggle;
            bindingKeys = builtins.attrNames (layer.bindings or { });
          in
          holdKeys ++ toggleKeys ++ bindingKeys
        )
        (builtins.attrValues layers);
    in
    map toKanataKey layerKeys;

  # Parse a "super + c" style combo into { mod, key }
  parseUniversalCombo = combo:
    let
      parts = map lib.trim (lib.splitString "+" combo);
      lower = map lib.toLower parts;
    in
    if builtins.length parts == 2 then {
      mod = builtins.head lower;
      key = lib.toLower (lib.last parts);
    }
    else null;

  # Collect letter keys from universal remaps (Super+letter → Ctrl+letter)
  collectUniversalKeys = universal:
    let
      entries = mapAttrsToList
        (_: entry:
          let
            parsed = parseUniversalCombo (entry.from or "");
            toParsed = parseUniversalCombo (entry.to or "");
          in
          if parsed != null && toParsed != null && parsed.mod == "super"
          then { src = parsed.key; dst = toParsed; }
          else null
        )
        universal;
    in
    builtins.filter (x: x != null) entries;

  # Generate a kanata layer definition
  mkLayerDef = name: layer: sourceKeys:
    let
      bindingMap = builtins.listToAttrs (
        mapAttrsToList
          (src: dst: {
            name = toKanataKey src;
            value = toKanataKey dst;
          })
          (layer.bindings or { })
      );
      keyMappings = map
        (srcKey:
          bindingMap.${srcKey} or "_"
        )
        sourceKeys;
    in
    "(deflayer ${name}\n  ${concatStringsSep " " keyMappings}\n)";

  # Generate the default layer with tap-hold and pass-through
  mkDefaultLayer = layers: sourceKeys:
    let
      activationMap = builtins.listToAttrs (
        lib.concatMap
          (entry:
            let
              inherit (entry) name;
              layer = entry.value;
              tapAction = layer.tapAction or null;
              hold = layer.hold or null;
              toggle = layer.toggle or null;
            in
            (lib.optional (hold != null) {
              name = toKanataKey hold;
              value =
                let
                  ms = toString (layer.tapHoldMs or 200);
                  hasBindings = (layer.bindings or { }) != { };
                in
                if tapAction != null && hasBindings then
                  "(tap-hold-release ${ms} ${ms} ${toKanataKey tapAction} (layer-toggle ${name}))"
                else if tapAction != null then
                # No hold bindings — just remap the key directly
                  toKanataKey tapAction
                else
                  "(layer-toggle ${name})";
            })
            ++ (lib.optional (toggle != null && hold == null) {
              name = toKanataKey toggle;
              value =
                let
                  ms = toString (layer.tapHoldMs or 200);
                  hasBindings = (layer.bindings or { }) != { };
                in
                if tapAction != null && hasBindings then
                  "(tap-hold-release ${ms} ${ms} ${toKanataKey tapAction} (layer-toggle ${name}))"
                else if tapAction != null then
                  toKanataKey tapAction
                else
                  "(layer-toggle ${name})";
            })
          )
          (mapAttrsToList (n: v: { name = n; value = v; }) layers)
      );
      keyMappings = map
        (srcKey:
          activationMap.${srcKey} or "_"
        )
        sourceKeys;
    in
    "(deflayer default\n  ${concatStringsSep " " keyMappings}\n)";

  # Generate defoverrides for Super+key → Ctrl+key (direct key-to-key)
  mkSuperOverrides = universalEntries:
    let
      overrides = map
        (entry:
          let
            dstMod =
              if entry.dst.mod == "ctrl" then "lctl"
              else if entry.dst.mod == "shift" then "lsft"
              else if entry.dst.mod == "alt" then "lalt"
              else entry.dst.mod;
          in
          "  (lmet ${toKanataKey entry.src}) (${dstMod} ${toKanataKey entry.dst.key})"
        )
        universalEntries;
    in
    if overrides == [ ] then ""
    else "(defoverrides\n${concatStringsSep "\n" overrides}\n)";

  # Main generator: produces kanata config string
  generate = cfg:
    let
      layers = cfg.layers or { };
      universal = cfg.universal or { };

      # Layer keys
      layerSourceKeys = collectLayerSourceKeys layers;

      # Universal Super→Ctrl entries
      universalEntries = collectUniversalKeys universal;

      hasLayers = layerSourceKeys != [ ];
      hasUniversal = universalEntries != [ ];
    in
    if !hasLayers && !hasUniversal then
      null
    else
      let
        # Layer section
        defsrc = lib.optionalString hasLayers
          "(defsrc\n  ${concatStringsSep " " (lib.unique layerSourceKeys)}\n)";
        defaultLayer = lib.optionalString hasLayers
          (mkDefaultLayer layers (lib.unique layerSourceKeys));
        layersWithBindings = lib.filterAttrs (_: l: (l.bindings or { }) != { }) layers;
        layerDefs = lib.optionalString (layersWithBindings != { }) (
          concatStringsSep "\n\n" (
            mapAttrsToList
              (name: layer:
                mkLayerDef name layer (lib.unique layerSourceKeys)
              )
              layersWithBindings
          )
        );

        # Super→Ctrl section (uses defoverrides, no defsrc needed)
        overrides = mkSuperOverrides universalEntries;
      in
      ''
        ;; Generated by Vogix keybinding module
        ;; Do not edit manually — configure via programs.vogix.keybindings

        ${defsrc}

        ${defaultLayer}

        ${layerDefs}

        ${overrides}
      '';

in
{
  inherit generate;
}

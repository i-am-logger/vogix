# The vogix SDDM greeter theme, installed the nixpkgs way
# ($out/share/sddm/themes/vogix — sddm-astronaut / where-is-my-sddm-theme
# precedent): Main.qml + metadata.desktop from desktop/Greeter/sddm, and a
# theme.conf rendered from the palette the caller passes. SDDM exposes the
# [General] keys to the QML as `config.<key>` — that is how the semantic
# slots reach the greeter with no sed sentinels and no recolored bitmaps.
{ lib
, runCommand
, conf ? { }
}:

let
  src = builtins.path {
    path = ../../desktop/Greeter/sddm;
    name = "vogix-sddm-theme-src";
  };
  confFile =
    if conf == { } then null
    else
      builtins.toFile "vogix-sddm-theme.conf" (
        "[General]\n"
        + lib.concatStrings (lib.mapAttrsToList (k: v: "${k}=${toString v}\n") conf)
      );
in
runCommand "vogix-sddm-theme"
{
  meta = {
    description = "Vogix-themed SDDM greeter theme";
    license = lib.licenses.cc-by-nc-sa-40;
  };
} ''
  t=$out/share/sddm/themes/vogix
  mkdir -p $t
  install -m644 ${src}/Main.qml $t/Main.qml
  install -m644 ${src}/metadata.desktop $t/metadata.desktop
  install -m644 ${if confFile != null then confFile else "${src}/theme.conf"} $t/theme.conf
''

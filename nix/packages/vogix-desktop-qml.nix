# The vogix desktop shell's v1 rendering layer — the QML tree, packaged as a
# quickshell config directory. Pure data (no build): quickshell loads
# `shell.qml` and resolves the local `Vogix` module from the same root.
# The v2 Rust vogix-desktop replaces this package wholesale; the contract
# files, verbs and unit name stay.
{ lib, runCommand, qt6 }:

runCommand "vogix-desktop-qml"
{
  src = builtins.path {
    path = ../../desktop;
    name = "vogix-desktop-qml-src";
    # Greeter/ is SDDM's tree, not quickshell's: it rides its own package
    # (vogix-sddm-theme) and leans on SDDM context properties (config,
    # userModel, sddm) that only exist inside SDDM's engine.
    filter = path: _type: builtins.baseNameOf path != "Greeter";
  };
  nativeBuildInputs = [ qt6.qtshadertools ];
  meta = {
    description = "vogix desktop shell (quickshell rendering layer)";
    license = lib.licenses.cc-by-nc-sa-40;
  };
} ''
  cp -r $src $out
  # Qt6's RHI wants precompiled shader packs: bake the live-background
  # fragment shader for every backend ShaderEffect can meet.
  chmod -R u+w $out
  qsb --glsl "100 es,120,150" --hlsl 50 --msl 12 \
    -o $out/data/aurora.frag.qsb $out/data/aurora.frag
''

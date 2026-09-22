# Unit tests for the desktop shell's pure logic: the parsers and policies
# in desktop/Services/lib/, run by Qt Quick Test on the offscreen platform,
# and the sysfs probe scripts in desktop/data/, run against fixture trees.
# They pin behaviour the cage smoke cannot reach (no GPU, no hwmon, no
# system bus in the build sandbox).
{ pkgs }:

pkgs.runCommand "vogix-desktop-logic"
{
  nativeBuildInputs = [ pkgs.qt6.qtdeclarative ];
  lib = ../../desktop/Services/lib;
  data = ../../desktop/data;
  tests = ./.;
} ''
  sh "$tests/probes.sh" "$data"

  # The tests import the libraries by their in-repo relative path; stage
  # the same layout.
  mkdir -p stage/desktop/Services stage/tests
  cp -r "$lib" stage/desktop/Services/lib
  cp -r "$tests" stage/tests/desktop

  export HOME=$TMPDIR
  export LC_ALL=C.UTF-8
  export QT_QPA_PLATFORM=offscreen
  export QT_PLUGIN_PATH=${pkgs.qt6.qtbase}/${pkgs.qt6.qtbase.qtPluginPrefix}
  qmltestrunner -input stage/tests/desktop \
    -import ${pkgs.qt6.qtdeclarative}/${pkgs.qt6.qtbase.qtQmlPrefix}
  touch $out
''

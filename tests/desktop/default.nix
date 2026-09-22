# Unit tests for the desktop shell's pure logic, run by Qt Quick Test on the
# offscreen platform: the parsers and policies in Services/lib/ and the
# quickshell-free QML types (Ballistics, Lease, Placement), both taken from
# the packaged tree. The sysfs probe scripts in data/ run against fixture
# trees. They pin behaviour the cage smoke cannot reach (no GPU, no hwmon,
# no system bus in the build sandbox).
{ pkgs, qml }:

pkgs.runCommand "vogix-desktop-logic"
{
  nativeBuildInputs = [ pkgs.qt6.qtdeclarative ];
  inherit qml;
  tests = ./.;
} ''
  sh "$tests/probes.sh" "$qml/data"

  # One staged root serves both import styles: the `../../desktop/...`
  # script imports resolve to stage/desktop, and the `qs.` module imports to
  # stage/qs, the way quickshell maps them. Both are the package.
  mkdir -p stage/tests
  ln -s "$qml" stage/desktop
  ln -s "$qml" stage/qs
  cp -r "$tests" stage/tests/desktop

  export HOME=$TMPDIR
  export LC_ALL=C.UTF-8
  export QT_QPA_PLATFORM=offscreen
  export QT_PLUGIN_PATH=${pkgs.qt6.qtbase}/${pkgs.qt6.qtbase.qtPluginPrefix}
  qmltestrunner -input stage/tests/desktop \
    -import "$PWD/stage" \
    -import ${pkgs.qt6.qtdeclarative}/${pkgs.qt6.qtbase.qtQmlPrefix}

  # The VU meters and the spectrum take their curve from Ballistics alone:
  # neither may carry its own copy of the constants tst_ballistics pins.
  for f in Services/Peaks.qml Services/Cava.qml; do
    grep -q 'Ballistics\.advance' "$qml/$f" \
      || { echo "$f does not advance through Ballistics"; exit 1; }
    if grep -nE '(^|[^0-9.])(3\.0|0\.53|0\.75)([^0-9]|$)' "$qml/$f"; then
      echo "$f carries its own ballistics constants"; exit 1
    fi
  done
  touch $out
''

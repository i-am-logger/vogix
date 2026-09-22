# The shell actually RUNS: a headless cage compositor hosts the real
# quickshell on the real QML, exactly as the vogix-desktop unit starts it
# (its ExecStart and Environment, runtime PATH wrapper included), on a
# private D-Bus session bus. The verbs are driven through the real
# `vogix desktop` CLI wherever one exists, so every call crosses the same
# CLI → IPC → shell path a keybinding takes.
#
# The layout is the shipped default (desktop-json.pin.json) plus every
# registry widget that default leaves out, so each widget the shell ships
# instantiates here, and custom cells covering each of their triggers.
# Host tools the shell reads (hyprctl, systemctl, tailscale) are fixtures
# answering the way the real tools do, and a real MPRIS player (mpv)
# plays on the session bus. Every log the runs write passes a
# gate: no script error, binding problem, failed component, failed spawn
# or warning of the shell's own beyond the exact lines a fixture provokes.
#
# Compositor-, PipeWire- and NetworkManager-dependent behaviour is out of
# reach here (cage has no layer-shell, session lock or idle protocol);
# desktop-taps covers the taps against a real PipeWire.
{ pkgs, qsPkgs, home-manager, hmModule }:

let
  inherit (pkgs) lib;

  # The vogix-desktop unit for a default desktop user, built against the
  # same packages the shell runs on here.
  unit = (home-manager.lib.homeManagerConfiguration {
    pkgs = qsPkgs;
    modules = [
      hmModule
      {
        home = {
          username = "t";
          homeDirectory = "/home/t";
          stateVersion = "24.11";
        };
        programs.vogix = {
          enable = true;
          appearance = {
            theme = "yoga";
            variant = "night";
            prebuiltThemes = [ "yoga" ];
          };
          desktop.enable = true;
          enableDaemon = false;
        };
      }
    ];
  }).config.systemd.user.services.vogix-desktop;
  execStart = builtins.head (lib.toList unit.Service.ExecStart);
  # The unit's PATH wrapper (ExecStart's first word): the probes run
  # under it too, with the same tools on PATH.
  desktopEnv = builtins.head (lib.splitString " " execStart);
  unitEnv = lib.concatMapStrings (e: "export ${lib.escapeShellArg e}\n") unit.Service.Environment;

  pin = builtins.fromJSON (builtins.readFile ../modules/desktop/desktop-json.pin.json);
  registry = import ../modules/desktop/registry.nix;
  placed = lib.concatMap
    (edge: lib.concatMap (section: pin.bars.${edge}.layout.${section}) [ "start" "center" "end" ])
    (builtins.attrNames pin.bars);
  # Registry widgets the default layout does not place: all of them on the
  # top bar, and the ones that may sit on a rail on the right one too.
  extras = builtins.filter (n: !(builtins.elem n placed)) registry.names;
  railExtras = builtins.filter (n: !(builtins.elem n registry.horizontalOnly)) extras;

  fixture = lib.recursiveUpdate pin {
    # One mount every host has and one no host has: the mounts cell comes
    # up for the first and has no entry for the second.
    meters.mounts = [ "/" "/vogix-smoke-absent" ];
    # Custom cells over every trigger but the timer: first show (text,
    # json, a stream), a watched file's creation and change, and the IPC
    # refresh. @RT@ becomes the runtime dir once the file is in place.
    custom = {
      smoke = { title = "SMK"; command = "echo SMOKE-42"; };
      gauge = {
        command = "printf '%s' '{\"text\":\"J-7\",\"state\":\"danger\",\"meter\":0.5}'";
        output = "json";
      };
      stream = { command = "echo S-1; echo S-2"; stream = true; };
      watched = { command = "cat @RT@/smoke-watch"; watch = [ "@RT@/smoke-watch" ]; };
      counter = {
        command = "n=$(cat @RT@/smoke-count 2>/dev/null || echo 0); n=$((n + 1)); echo $n > @RT@/smoke-count; echo RUN-$n";
      };
    };
    bars = {
      top.layout.center = pin.bars.top.layout.center ++ extras ++ [ "custom/smoke" "custom/gauge" ];
      right.layout.center = pin.bars.right.layout.center ++ railExtras
        ++ [ "custom/watched" "custom/counter" "custom/stream" ];
    };
  };
  desktopJson = builtins.toJSON fixture;
  # The same layout with the scanline texture on, notification cards
  # included.
  scanlinesJson = builtins.toJSON (lib.recursiveUpdate fixture { background.scanlines = true; });
  # A schema-1 desktop.json (the single-bar shape, no `bars`). The shell
  # reads schema 2 only: it must stay up, render no bar, and say why.
  schema1Json = builtins.toJSON {
    schema = 1;
    font = { family = "monospace"; size = 13; };
    bar = {
      enable = true;
      position = "top";
      height = 32;
      layout = { left = [ "clock" ]; center = [ ]; right = [ ]; };
    };
    inherit (pin) surfaces;
  };

  themeJson = builtins.toJSON {
    schema = 1;
    theme = "smoke";
    variant = "night";
    scheme = "vogix16";
    polarity = "dark";
    backgrounds = [ ];
    palette = { base00 = "#101010"; };
    semantic = {
      active = "#1c1c1c";
      background = "#101010";
      background_selection = "#121212";
      background_surface = "#111111";
      danger = "#1b1b1b";
      foreground_border = "#141414";
      foreground_bright = "#171717";
      foreground_comment = "#131313";
      foreground_heading = "#161616";
      foreground_text = "#151515";
      highlight = "#1e1e1e";
      link = "#1d1d1d";
      notice = "#1a1a1a";
      special = "#1f1f1f";
      success = "#181818";
      warning = "#191919";
    };
  };

  # The input engine's mode table for the mode cell, and its current mode.
  inputJson = builtins.toJSON {
    modeColors.normal = { slot = "active"; label = "NRM-SMOKE"; };
  };

  # `hyprctl -j devices` and `-j activewindow` as Hyprland answers them
  # (HyprCtl.cpp's shapes); every other call fails as it does with no
  # compositor. The keyboard the LANG cell follows is neither first nor
  # main, and its active layout's name ("German") does not abbreviate to
  # its code, so only the right device plus the reported index yield
  # "active:de".
  hyprctlFixture =
    let
      devices = pkgs.writeText "hyprctl-devices.json" (builtins.toJSON {
        mice = [ ];
        keyboards = [
          {
            address = "0x1";
            name = "at-translated-set-2-keyboard";
            rules = "";
            model = "";
            layout = "us";
            variant = "";
            options = "";
            active_layout_index = 0;
            active_keymap = "English (US)";
            capsLock = false;
            numLock = false;
            main = true;
          }
          {
            address = "0x2";
            name = "vogix-input";
            rules = "";
            model = "";
            layout = "de,us";
            variant = "";
            options = "grp:alt_caps_toggle";
            active_layout_index = 0;
            active_keymap = "German";
            capsLock = false;
            numLock = false;
            main = false;
          }
        ];
        tablets = [ ];
        touch = [ ];
        switches = [ ];
      });
      activewindow = pkgs.writeText "hyprctl-activewindow.json" (builtins.toJSON {
        address = "0x3";
        mapped = true;
        hidden = false;
        at = [ 140 110 ];
        size = [ 800 600 ];
        workspace = { id = 1; name = "1"; };
        floating = false;
        monitor = 0;
        class = "smoke-class";
        title = "smoke window title";
        fullscreen = 0;
      });
    in
    pkgs.writeShellScriptBin "hyprctl" ''
      case "$*" in
        "-j devices") cat ${devices} ;;
        "-j activewindow") cat ${activewindow} ;;
        *) exit 1 ;;
      esac
    '';
  # tailscaled's start time as systemd reports it, and a connected tailnet.
  systemctlFixture = pkgs.writeShellScriptBin "systemctl" ''
    case "$*" in
      "show tailscaled --property=ActiveEnterTimestamp --value --timestamp=unix") echo @1790100514 ;;
      *) exit 1 ;;
    esac
  '';
  tailscaleFixture =
    let
      status = pkgs.writeText "tailscale-status.json" (builtins.toJSON {
        BackendState = "Running";
        TailscaleIPs = [ "100.64.0.1" ];
        Self = { DNSName = "smoke.tail0.ts.net."; Online = true; };
        CurrentTailnet = { Name = "smoke.example"; };
        Peer = { };
      });
    in
    pkgs.writeShellScriptBin "tailscale" ''
      if [ "$*" = "status --json" ]; then cat ${status}; else exit 1; fi
    '';
in
pkgs.runCommand "vogix-desktop-smoke"
{
  nativeBuildInputs = [
    pkgs.cage
    pkgs.dbus
    pkgs.jq
    pkgs.libnotify
    pkgs.mpv
    pkgs.playerctl
    qsPkgs.quickshell
    qsPkgs.vogix
    hyprctlFixture
    systemctlFixture
    tailscaleFixture
  ];
  qml = qsPkgs.vogix-desktop-qml;
  geometryProbe = ./desktop-geometry-probe.qml;
  pinJson = ../modules/desktop/desktop-json.pin.json;
  stateProbe = ./desktop-state-probe.qml;
  inherit themeJson desktopJson scanlinesJson schema1Json inputJson;
  passAsFile = [ "themeJson" "desktopJson" "scanlinesJson" "schema1Json" "inputJson" ];
} ''
  export HOME=$TMPDIR/home
  export XDG_CONFIG_HOME=$HOME/.config
  export XDG_STATE_HOME=$HOME/.local/state
  export XDG_RUNTIME_DIR=$TMPDIR/rt
  mkdir -p $XDG_CONFIG_HOME/vogix-desktop $XDG_CONFIG_HOME/quickshell $XDG_STATE_HOME/vogix/desktop $XDG_RUNTIME_DIR
  chmod 700 $XDG_RUNTIME_DIR
  # The `vogix` quickshell config, as home-manager registers it.
  ln -s $qml $XDG_CONFIG_HOME/quickshell/vogix
  cp $themeJsonPath $XDG_CONFIG_HOME/vogix-desktop/theme.json
  for f in desktop scanlines; do
    eval src=\$''${f}JsonPath
    sed "s|@RT@|$XDG_RUNTIME_DIR|g" $src > $TMPDIR/$f.json
  done
  cp $TMPDIR/desktop.json $XDG_STATE_HOME/vogix/desktop.json
  # The fixture is a document the check passes, as a generated one is.
  vogix desktop check --config $TMPDIR/desktop.json

  # Each probe replaces shell.qml in a copy of the tree, so `qs.`
  # resolves to the shipped modules around it.
  for probe in geometry state; do
    cp -r $qml $TMPDIR/$probe
    chmod -R u+w $TMPDIR/$probe
  done
  cp $geometryProbe $TMPDIR/geometry/shell.qml
  cp $stateProbe $TMPDIR/state/shell.qml

  cat > inner.sh <<'INNER'
  ${unitEnv}
  # No GPU and no GL in the build sandbox: Qt's software scene graph.
  export QT_QUICK_BACKEND=software
  R=$TMPDIR/result
  # The shell, as the unit runs it; LOG names its log.
  launch() {
    ${execStart} > $TMPDIR/$1 2>&1 &
    QSPID=$!
  }
  stop() {
    kill $QSPID 2>/dev/null
    wait $QSPID 2>/dev/null
  }
  # A verb, as keybindings and scripts call it; LABEL prefixes its reply.
  v() {
    label=$1
    shift
    vogix desktop "$@" 2>&1 | sed "s|^|$label |" >> $R
  }
  # A transport-only IPC function (no verb reaches it).
  ipc() {
    label=$1
    shift
    qs -c vogix ipc call "$@" 2>&1 | sed "s|^|$label |" >> $R
  }

  # An MPRIS player on the session bus before the shell starts: the
  # media cell comes up with a player to show and control.
  mpv --no-config --really-quiet --idle=no --loop=inf --ao=null --vo=null \
    --script=${pkgs.mpvScripts.mpris}/share/mpv/scripts/mpris.so \
    av://lavfi:sine=frequency=440 > $TMPDIR/mpv.log 2>&1 &
  MPVPID=$!
  launch qs.log
  sleep 5
  kill -0 $QSPID 2>/dev/null && echo ALIVE >> $R
  # The player changes state under the shell.
  playerctl pause
  sleep 1
  playerctl play
  sleep 1
  echo "media $(playerctl status 2>&1)" >> $R
  v status status
  v bar-hide-left bar hide left
  v bar-show bar show
  v bar-toggle bar toggle
  v bar-toggle-back bar toggle
  # A bar nobody can see samples nothing; shown again, it samples.
  v bars-hidden bar hide
  sleep 1
  v meters-hidden meters
  v bars-shown bar show
  sleep 2
  v meters-shown meters
  v reload reload
  ipc launcher launcher status
  v power power
  ipc power-status power status
  ipc power-close power close
  v panel-calendar panel calendar
  v panel-status panel
  v panel-out panel audio-out
  v panel-in panel audio-in
  v panel-tailscale panel tailscale
  v panel-close panel --close
  v stayawake stay-awake toggle
  v stayawake-status stay-awake status
  v nightlight nightlight status
  v gallery gallery
  ipc gallery-status gallery status
  v gallery-close gallery --close
  v reminders remind list
  vogix desktop stats > $TMPDIR/stats.json 2>&1
  v privacy privacy
  for cell in smoke gauge stream watched; do
    v custom-$cell custom status $cell
  done
  v custom-undefined custom status undefined
  echo W-1 > $XDG_RUNTIME_DIR/smoke-watch
  sleep 1
  v custom-watched-created custom status watched
  echo W-2 > $XDG_RUNTIME_DIR/smoke-watch
  sleep 1
  v custom-watched-changed custom status watched
  v custom-counter-refresh custom refresh counter
  sleep 1
  v custom-counter custom status counter
  v keyboard keyboard
  # The input engine's lock document, handled as the engine does:
  # written tmp + rename, rewritten the same way, removed on stop. The
  # LANG cell must follow every step through its file watch.
  put_locks() {
    printf '%s' "$1" > $XDG_STATE_HOME/vogix/input-locks.json.tmp
    mv $XDG_STATE_HOME/vogix/input-locks.json.tmp $XDG_STATE_HOME/vogix/input-locks.json
  }
  caps_until() {
    for _ in $(seq 50); do
      s=$(vogix desktop keyboard)
      case "$s" in *" caps:$1") break ;; esac
      sleep 0.1
    done
    echo "$2 $s" >> $R
  }
  put_locks '{"capsLock":true,"numLock":false,"scrollLock":null}'
  caps_until on locks-on
  put_locks '{"capsLock":false,"numLock":false,"scrollLock":null}'
  caps_until off locks-off
  put_locks '{"capsLock":null,"numLock":null,"scrollLock":null}'
  caps_until unknown locks-null
  put_locks '{"capsLock":true,"numLock":false,"scrollLock":null}'
  caps_until on locks-back
  rm $XDG_STATE_HOME/vogix/input-locks.json
  caps_until unknown locks-gone
  # Two notifications, a critical one whose body wraps: cards in the
  # popup column, and the live set mirrored to the state file.
  notify-send -u critical -a smoke-critical "Critical summary" \
    "a body long enough to wrap across more than one line of the card, which exercises the wrapped text height path"
  notify-send -a smoke-normal -t 60000 "Normal summary" "short body"
  sleep 3
  ipc notify-count notify count
  cp $XDG_STATE_HOME/vogix/desktop/notifications.json $TMPDIR/notifications-1.json
  stop
  kill $MPVPID 2>/dev/null

  # Restart run: the live notifications come back, each with the time it
  # arrived.
  launch qs-restart.log
  sleep 4
  ipc restart-notify-count notify count
  cp $XDG_STATE_HOME/vogix/desktop/notifications.json $TMPDIR/notifications-2.json
  stop

  # Scanlines run: the texture on the bars and the restored cards.
  cp $TMPDIR/scanlines.json $XDG_STATE_HOME/vogix/desktop.json
  launch qs-scanlines.log
  sleep 4
  kill -0 $QSPID 2>/dev/null && echo SCANLINES-ALIVE >> $R
  ipc scanlines-notify-count notify count
  stop
  # Geometry run, on the shipped default desktop.json: the probe exits on
  # its own with its verdict; the timeout only bounds a probe that never
  # completes.
  install -m 644 $pinJson $XDG_STATE_HOME/vogix/desktop.json
  timeout 60 ${desktopEnv} qs -p $TMPDIR/geometry > $TMPDIR/qs-geometry.log 2>&1
  echo "GEOMETRY-EXIT $?" >> $R
  cp $TMPDIR/desktop.json $XDG_STATE_HOME/vogix/desktop.json

  # State run, with the scanline texture on: the window title, the mode
  # cell's table and then its loss, and the restored cards' texture.
  cp $TMPDIR/scanlines.json $XDG_STATE_HOME/vogix/desktop.json
  cp $inputJsonPath $XDG_STATE_HOME/vogix/input.json
  echo normal > $XDG_STATE_HOME/vogix/current-mode
  timeout 60 ${desktopEnv} qs -p $TMPDIR/state > $TMPDIR/qs-state.log 2>&1
  echo "STATE-EXIT $?" >> $R

  # Rejection run: a schema-1 desktop.json is refused, loudly, without
  # taking the shell down.
  cp $schema1JsonPath $XDG_STATE_HOME/vogix/desktop.json
  launch qs-schema1.log
  sleep 3
  kill -0 $QSPID 2>/dev/null && echo SCHEMA1-ALIVE >> $R
  v schema1 bar status
  stop
  INNER
  chmod +x inner.sh
  WLR_BACKENDS=headless WLR_LIBINPUT_NO_DEVICES=1 WLR_RENDERER=pixman \
    cage -- dbus-run-session --config-file=${pkgs.dbus}/share/dbus-1/session.conf -- ${pkgs.runtimeShell} ./inner.sh || true

  echo "── result:"; cat $TMPDIR/result || true
  echo "── stats:"; cat $TMPDIR/stats.json || true
  echo "── geometry:"; grep -hE "GEOMETRY|FOOTPRINT|FIT" $TMPDIR/qs-geometry.log || true
  echo "── state:"; grep -h 'STATE' $TMPDIR/qs-state.log || true
  echo "── notification arrival times:"
  jq -c '[.[].at]' $TMPDIR/notifications-1.json $TMPDIR/notifications-2.json || true

  # The log gate. A failure is a script error, a binding problem, a
  # component that did not load, a program the shell could not start, or
  # a warning the shell logged itself; only the exact lines a fixture
  # provokes are allowed, per log.
  failures='TypeError|ReferenceError|SyntaxError|RangeError|Binding loop|Unable to assign|Cannot assign|is not a type|is not installed|Failed to load configuration|Type [^ ]+ unavailable|Script [^ ]+ unavailable|\]: File not found|Process failed to start|^(WARN|ERROR|CRIT) +qml:'
  # The watched cell's command fails until its file exists.
  watched='WARN qml: vogix: custom/watched: exited 1'
  gate() {
    log=$TMPDIR/$1
    shift
    test -s $log || { echo "── $log is missing or empty"; return 1; }
    # quickshell colours its level and category names.
    lines=$(sed 's/\x1b\[[0-9;]*m//g; s/^ *//' $log | grep -E "$failures" || true)
    for allowed in "$@"; do
      lines=$(printf '%s\n' "$lines" | grep -vxF -- "$allowed" || true)
    done
    bad=$lines
    if [ -n "$bad" ]; then
      echo "── $log:"
      echo "$bad"
      return 1
    fi
  }
  clean=0
  gate qs.log "$watched" || clean=1
  gate qs-restart.log || clean=1
  gate qs-scanlines.log || clean=1
  gate qs-geometry.log || clean=1
  gate qs-state.log || clean=1
  gate qs-schema1.log 'WARN qml: vogix: desktop.json schema 1 is not supported (this shell reads schema 2); rebuild to regenerate it' || clean=1
  test $clean = 0

  r() { grep -qxF -- "$1" $TMPDIR/result || { echo "missing result line: $1"; exit 1; }; }
  r ALIVE
  r 'media Playing'
  r 'status shell: running'
  r 'status bar: top:shown bottom:shown left:shown right:shown'
  r 'bar-hide-left top:shown bottom:shown left:hidden right:shown'
  r 'bar-show top:shown bottom:shown left:shown right:shown'
  r 'bar-toggle top:hidden bottom:hidden left:hidden right:hidden'
  r 'bar-toggle-back top:shown bottom:shown left:shown right:shown'
  r 'meters-hidden spectrum:off scope:off vu-out:off vu-mic:off stats:none'
  grep -qE '^meters-shown .* stats:[a-z]' $TMPDIR/result
  r 'launcher closed'
  r 'power open'
  r 'power-status open'
  r 'panel-calendar calendar'
  r 'panel-status calendar'
  r 'panel-out audio-out'
  r 'panel-in audio-in'
  r 'panel-tailscale tailscale'
  r 'stayawake on'
  r 'stayawake-status on'
  r 'nightlight off'
  r 'gallery open'
  r 'gallery-status open'
  r 'gallery-close closed'
  r 'reminders no reminders'
  # The live bars' stats have samples: memory is read by a FileView that
  # reload()s on a tick, and reload() never performs a file's first load,
  # so without a preload it would stay at nothing. The mounts cell shows
  # "/" unless the root is RAM-backed, and never the path this host lacks.
  jq -e '.memory > 0 and .cpu != null' $TMPDIR/stats.json
  jq -e '.gauges == (if .rootInMemory then [] else ["/"] end)
    and (.mounts | has("/vogix-smoke-absent") | not)' $TMPDIR/stats.json
  r 'privacy mic:off screen:off'
  r 'custom-smoke SMOKE-42'
  r 'custom-gauge J-7'
  r 'custom-stream S-2'
  r 'custom-watched failed: exited 1'
  r 'custom-watched-created W-1'
  r 'custom-watched-changed W-2'
  r 'custom-counter-refresh refreshing'
  r 'custom-counter RUN-2'
  # A name desktop.json does not define is the caller's error.
  r 'custom-undefined [ERROR] config error: unknown custom cell: undefined'
  r 'keyboard device:vogix-input layouts:de,us active:de caps:unknown'
  r 'locks-on device:vogix-input layouts:de,us active:de caps:on'
  grep -q '^locks-off .* caps:off$' $TMPDIR/result
  grep -q '^locks-null .* caps:unknown$' $TMPDIR/result
  grep -q '^locks-back .* caps:on$' $TMPDIR/result
  grep -q '^locks-gone .* caps:unknown$' $TMPDIR/result
  # The tailnet link's connection record: the shell saw it connected on
  # its first sample, so the start is inexact and bounded by the daemon's.
  jq -e '.exact == false and .daemonStartMs == 1790100514000' \
    $XDG_RUNTIME_DIR/vogix/desktop/tailscale-connection.json
  # Notifications: both cards up and mirrored with their arrival time, the
  # same two restored with the same times after a restart, and with the
  # scanline texture on.
  r 'notify-count 2'
  jq -e 'length == 2 and all(.[]; (.at | type) == "number")' $TMPDIR/notifications-1.json
  r 'restart-notify-count 2'
  test "$(jq -c '[.[].at]' $TMPDIR/notifications-1.json)" = "$(jq -c '[.[].at]' $TMPDIR/notifications-2.json)"
  r SCANLINES-ALIVE
  r 'scanlines-notify-count 2'
  # A canvas instrument never lays out collapsed nor resizes when audio
  # arrives, and the default layout fits its bars: the probe's verdict
  # over every registry widget on both bar axes and every default cell,
  # with the oscilloscope measured and each bar's cells fitted.
  r 'GEOMETRY-EXIT 0'
  grep -q 'GEOMETRY bottom oscilloscope [1-9][0-9.]*x[1-9][0-9.]*$' $TMPDIR/qs-geometry.log
  for edge in top bottom left right; do
    grep -q "FIT $edge " $TMPDIR/qs-geometry.log
  done
  grep -q 'FIT bottom oscilloscope ' $TMPDIR/qs-geometry.log
  # The window title from hyprctl; the mode label from input.json, then
  # the bare mode once input.json cannot be read; the texture on both
  # restored cards.
  r 'STATE-EXIT 0'
  grep -q 'STATE window-title smoke window title$' $TMPDIR/qs-state.log
  grep -q 'STATE mode-label NRM-SMOKE$' $TMPDIR/qs-state.log
  grep -q 'STATE mode-label-after-failure normal$' $TMPDIR/qs-state.log
  grep -q 'STATE card-scanlines 2/2$' $TMPDIR/qs-state.log
  r SCHEMA1-ALIVE
  r 'schema1 top:off bottom:off left:off right:off'
  grep -q 'desktop.json schema 1 is not supported' $TMPDIR/qs-schema1.log
  touch $out
''

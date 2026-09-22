# The audio taps' lifecycle against a REAL PipeWire daemon: the shell runs
# under a headless cage compositor (as in desktop-smoke), PipeWire runs
# from a private config with one null sink published as the default, and
# `meters status` (the IPC behind `vogix desktop meters`) plus the process
# table are the observations. Proven here:
#
# - a shell that starts before PipeWire waits (no tap is launched) and
#   starts both taps once PipeWire and a default sink appear;
# - a tap that dies is relaunched;
# - a PipeWire restart stops the taps and brings them back.
#
# No session manager runs, so nothing links the taps: they sit connected
# and idle, which is all a lifecycle test needs.
{ pkgs, qsPkgs }:

let
  themeJson = builtins.toJSON {
    schema = 1;
    theme = "taps";
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

  # One spectrum and one scope, nothing else: every tap has one consumer.
  desktopJson = builtins.toJSON {
    schema = 2;
    font = { family = "monospace"; size = 16; };
    bars = {
      top = {
        enable = true;
        size = 36;
        layout = { start = [ "spectrum-mini" ]; center = [ ]; end = [ ]; };
      };
      bottom = {
        enable = true;
        size = 96;
        layout = { start = [ ]; center = [ "oscilloscope" ]; end = [ ]; };
      };
      left = { enable = false; size = 36; layout = { start = [ ]; center = [ ]; end = [ ]; }; };
      right = { enable = false; size = 36; layout = { start = [ ]; center = [ ]; end = [ ]; }; };
    };
    meters.spectrum = { enable = true; bars = 8; };
    notifications.enable = false;
    background.enable = false;
    decorations.focusBrackets = false;
    surfaces.bar = {
      background = { slot = "background"; alpha = 0.92; };
      foreground = { slot = "foreground_text"; alpha = 1.0; };
      muted = { slot = "foreground_comment"; alpha = 1.0; };
      accent = { slot = "active"; alpha = 1.0; };
      border = { slot = "foreground_border"; alpha = 1.0; };
      urgent = { slot = "danger"; alpha = 1.0; };
    };
  };

  # A daemon with no hardware: a null sink, published as the default the
  # way a session manager would, and the dummy driver that clocks it.
  pipewireConf = pkgs.writeText "taps-pipewire.conf" ''
    context.properties = {
      core.daemon = true
      core.name = pipewire-0
      link.max-buffers = 16
      support.dbus = false
    }
    context.spa-libs = {
      audio.convert.* = audioconvert/libspa-audioconvert
      support.* = support/libspa-support
    }
    context.modules = [
      { name = libpipewire-module-protocol-native }
      { name = libpipewire-module-metadata }
      { name = libpipewire-module-spa-node-factory }
      { name = libpipewire-module-client-node }
      { name = libpipewire-module-adapter }
      { name = libpipewire-module-link-factory }
      { name = libpipewire-module-access }
    ]
    context.objects = [
      { factory = spa-node-factory
        args = {
          factory.name = support.node.driver
          node.name = Dummy-Driver
          priority.driver = 20000
        }
      }
      { factory = adapter
        args = {
          factory.name = support.null-audio-sink
          node.name = taps-sink
          media.class = Audio/Sink
          audio.position = [ FL FR ]
          monitor.channel-volumes = true
        }
      }
      { factory = metadata
        args = {
          metadata.name = default
          metadata.values = [
            { key = default.audio.sink type = "Spa:String:JSON" value = { name = taps-sink } }
          ]
        }
      }
    ]
  '';

  inner = pkgs.writeShellScript "taps-inner" ''
    set -u
    export QS_NO_RELOAD_POPUP=1 QS_DISABLE_FILE_WATCHER=1 QT_QUICK_BACKEND=software
    # As the vogix-desktop unit sets it: a PipeWire that is not up yet is
    # waited for.
    export QS_PIPEWIRE_IMMEDIATE_RECONNECT=1
    out=$TMPDIR/result
    note() { echo "$*" >> "$out"; }
    meters() { qs -p "$qml" ipc call meters status 2>&1; }
    # Polls `meters status` every 0.5 s until it contains $1, for up to $2 s.
    await() {
      local tries=$(( $2 * 2 )) s=""
      while [ "$tries" -gt 0 ]; do
        s=$(meters)
        case "$s" in *"$1"*) note "ok: $1"; return 0 ;; esac
        sleep 0.5
        tries=$(( tries - 1 ))
      done
      note "FAIL: no '$1' within $2 s (last: $s)"
      return 1
    }
    startpw() {
      pipewire -c ${pipewireConf} >> "$TMPDIR/pipewire.log" 2>&1 &
      PWPID=$!
    }

    qs -p "$qml" > "$TMPDIR/qs.log" 2>&1 &
    QSPID=$!
    sleep 3

    # 1. Started before PipeWire: both taps wait, neither is launched.
    await "spectrum:waiting scope:waiting" 10
    if pgrep -x cava > /dev/null || pgrep -x pw-record > /dev/null; then
      note "FAIL: a tap was launched without PipeWire"
    fi

    # 2. PipeWire and its default sink appear: both taps start.
    startpw
    await "spectrum:running scope:running" 20
    cava1=$(pgrep -x cava) pw1=$(pgrep -x pw-record)
    note "pids: cava=$cava1 pw-record=$pw1"

    # 3. A tap that dies comes back as a new process.
    kill $cava1 $pw1
    sleep 0.5
    await "spectrum:running scope:running" 10
    cava2=$(pgrep -x cava) pw2=$(pgrep -x pw-record)
    note "pids: cava=$cava2 pw-record=$pw2"
    if [ -n "$cava2" ] && [ "$cava2" != "$cava1" ] && [ -n "$pw2" ] && [ "$pw2" != "$pw1" ]; then
      note "ok: relaunched"
    else
      note "FAIL: not relaunched"
    fi

    # 4. PipeWire goes away: the taps stop and wait for it.
    kill $PWPID
    wait $PWPID
    await "spectrum:waiting scope:waiting" 10
    sleep 1
    if pgrep -x cava > /dev/null || pgrep -x pw-record > /dev/null; then
      note "FAIL: a tap outlived PipeWire"
    else
      note "ok: stopped with PipeWire"
    fi

    # 5. PipeWire comes back: so do the taps.
    startpw
    await "spectrum:running scope:running" 20

    kill $QSPID $PWPID 2>/dev/null
    wait
  '';
in
pkgs.runCommand "vogix-desktop-taps"
{
  nativeBuildInputs = [
    pkgs.cage
    qsPkgs.quickshell
    pkgs.pipewire
    pkgs.cava
    pkgs.procps
  ];
  qml = qsPkgs.vogix-desktop-qml;
  inherit themeJson desktopJson;
  passAsFile = [ "themeJson" "desktopJson" ];
} ''
  export HOME=$TMPDIR/home
  export XDG_CONFIG_HOME=$HOME/.config
  export XDG_STATE_HOME=$HOME/.local/state
  export XDG_RUNTIME_DIR=$TMPDIR/rt
  mkdir -p $XDG_CONFIG_HOME/vogix-desktop $XDG_STATE_HOME/vogix $XDG_RUNTIME_DIR
  chmod 700 $XDG_RUNTIME_DIR
  cp $themeJsonPath $XDG_CONFIG_HOME/vogix-desktop/theme.json
  cp $desktopJsonPath $XDG_STATE_HOME/vogix/desktop.json

  WLR_BACKENDS=headless WLR_LIBINPUT_NO_DEVICES=1 WLR_RENDERER=pixman \
    cage -- ${inner} || true

  echo "── result:"; cat $TMPDIR/result || true
  echo "── qs.log (vogix lines):"; grep 'vogix' $TMPDIR/qs.log || true
  if grep -q '^FAIL' $TMPDIR/result; then
    echo "── qs.log:"; cat $TMPDIR/qs.log
    echo "── pipewire.log:"; cat $TMPDIR/pipewire.log || true
    exit 1
  fi
  test "$(grep -c '^ok: ' $TMPDIR/result)" -eq 7
  touch $out
''

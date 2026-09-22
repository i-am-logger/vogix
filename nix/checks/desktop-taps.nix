# The audio taps' lifecycle against a REAL PipeWire daemon: the shell runs
# under a headless cage compositor (as in desktop-smoke), PipeWire runs
# from a private config with one null sink published as the default, and
# `meters status` (the IPC behind `vogix desktop meters`) plus the process
# table are the observations. Proven here:
#
# - with nothing playing — PipeWire down or up — no tap is launched;
# - a playback stream starts both taps and the output VU, and they stop
#   when it ends;
# - the shell's own taps and VU monitors never light the privacy cell's
#   microphone flag; another program's capture stream does, until it ends;
# - a tap that dies is relaunched;
# - without a default sink the taps wait, and start when one appears;
# - a hidden bar's taps, VU monitors and stat samplers stop, per edge, and
#   come back when it is shown;
# - a PipeWire restart stops the taps and brings them back;
# - launched as the unit launches it, quickshell writes no DEBUG records.
#
# No session manager runs, so nothing links the streams: the taps and the
# playback stream sit connected and idle, which is all a lifecycle test
# needs.
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

  # One consumer per source, split over two bars so hiding one edge
  # stops exactly its own sources.
  desktopJson = builtins.toJSON {
    schema = 2;
    font = { family = "monospace"; size = 16; };
    bars = {
      top = {
        enable = true;
        size = 36;
        layout = { start = [ "spectrum-mini" ]; center = [ ]; end = [ "uptime" ]; };
      };
      bottom = {
        enable = true;
        size = 96;
        layout = { start = [ "stat-cpu" ]; center = [ "oscilloscope" ]; end = [ "vu-out" "vu-mic" ]; };
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
    # Fails the step if any tap process is alive.
    notaps() {
      sleep 1
      if pgrep -x cava > /dev/null || pgrep -x pw-record > /dev/null; then
        note "FAIL: a tap is running: $1"
      else
        note "ok: no taps: $1"
      fi
    }
    startpw() {
      pipewire -c ${pipewireConf} >> "$TMPDIR/pipewire.log" 2>&1 &
      PWPID=$!
    }
    # A playback stream: what the taps and the output VU wait for.
    play() {
      pw-play --raw --format=s16 --rate=48000 --channels=2 /dev/zero >> "$TMPDIR/pipewire.log" 2>&1 &
      PLAYPID=$!
    }
    # Polls `privacy status` every 0.5 s until it is $1, for up to $2 s.
    await_privacy() {
      local tries=$(( $2 * 2 )) s=""
      while [ "$tries" -gt 0 ]; do
        s=$(qs -p "$qml" ipc call privacy status 2>&1)
        [ "$s" = "$1" ] && { note "ok: privacy $1"; return 0; }
        sleep 0.5
        tries=$(( tries - 1 ))
      done
      note "FAIL: privacy not '$1' within $2 s (last: $s)"
      return 1
    }
    running="spectrum:running scope:running vu-out:on"
    idle="spectrum:idle scope:idle vu-out:idle"

    # Launched as the unit launches it (desktop.detailedLogs = false).
    qs -p "$qml" --no-detailed-logs > "$TMPDIR/qs.log" 2>&1 &
    QSPID=$!
    sleep 3

    # 1. Before PipeWire, and with PipeWire but nothing playing: idle.
    await "$idle vu-mic:on" 10
    notaps "before PipeWire"
    startpw
    sleep 2
    await "$idle vu-mic:on" 10
    notaps "nothing playing"

    # 2. Playback starts both taps and the output VU.
    play
    await "$running" 20

    # 2b. The taps (cava, the scope's pw-record) and the peak monitors
    # capture the output monitor, not a microphone: with all of them
    # running the privacy cell's microphone flag stays off. Another
    # program's capture stream turns it on, and its end turns it off.
    await_privacy "mic:off screencast:off" 5
    pw-cat --record --raw --format=s16 --rate=48000 --channels=2 /dev/null >> "$TMPDIR/pipewire.log" 2>&1 &
    CAPPID=$!
    await_privacy "mic:on screencast:off" 10
    kill $CAPPID
    wait $CAPPID
    await_privacy "mic:off screencast:off" 10
    cava1=$(pgrep -x cava) pw1=$(pgrep -x pw-record)

    # 3. A tap that dies comes back as a new process.
    kill $cava1 $pw1
    sleep 0.5
    await "$running" 10
    cava2=$(pgrep -x cava) pw2=$(pgrep -x pw-record)
    note "pids: cava $cava1 -> $cava2, pw-record $pw1 -> $pw2"
    if [ -n "$cava2" ] && [ "$cava2" != "$cava1" ] && [ -n "$pw2" ] && [ "$pw2" != "$pw1" ]; then
      note "ok: relaunched"
    else
      note "FAIL: not relaunched"
    fi

    # 4. No default sink: the taps wait; one appears: they start.
    pw-metadata -n default -d 0 default.audio.sink >> "$TMPDIR/pipewire.log" 2>&1
    await "spectrum:waiting scope:waiting" 10
    notaps "no default sink"
    pw-metadata -n default 0 default.audio.sink '{ "name": "taps-sink" }' Spa:String:JSON >> "$TMPDIR/pipewire.log" 2>&1
    await "$running" 10

    # 5. Every source runs while its bar is shown; hiding an edge stops
    # exactly that edge's sources, hiding all stops everything, and
    # showing them again brings it all back.
    await "$running vu-mic:on stats:cpu,uptime" 5
    qs -p "$qml" ipc call bar hide bottom > /dev/null
    await "spectrum:running scope:off vu-out:off vu-mic:off stats:uptime" 5
    sleep 1
    if pgrep -x pw-record > /dev/null || ! pgrep -x cava > /dev/null; then
      note "FAIL: hiding the bottom bar did not stop exactly the scope tap"
    else
      note "ok: bottom hidden"
    fi
    qs -p "$qml" ipc call bar hide all > /dev/null
    await "spectrum:off scope:off vu-out:off vu-mic:off stats:none" 5
    notaps "all bars hidden"
    qs -p "$qml" ipc call bar unhide all > /dev/null
    await "$running vu-mic:on stats:cpu,uptime" 10

    # 6. Playback ends: back to idle, no taps.
    kill $PLAYPID
    wait $PLAYPID
    await "$idle" 10
    notaps "playback ended"

    # 7. A PipeWire restart: everything stops with it and returns with it.
    play
    await "$running" 10
    kill $PWPID
    wait $PWPID $PLAYPID
    await "$idle" 10
    notaps "PipeWire stopped"
    startpw
    sleep 1
    play
    await "$running" 20

    kill $QSPID $PWPID $PLAYPID 2>/dev/null
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
  # --no-detailed-logs is accepted and keeps quickshell's DEBUG records
  # out of the log: about 3 KB of warnings for this run, against ~64 KB
  # with detailed logs on.
  qslog=$(find $XDG_RUNTIME_DIR/quickshell -name log.qslog | head -n 1)
  echo "── detailed log: $(stat -c %s "$qslog") bytes"
  test "$(stat -c %s "$qslog")" -lt 16384
  # The QML behind the taps and the leases must run clean.
  if grep -E 'TypeError|ReferenceError|Binding loop|Unable to assign' $TMPDIR/qs.log; then
    echo "script errors in qs.log"
    exit 1
  fi
  if grep -q '^FAIL' $TMPDIR/result; then
    echo "── qs.log:"; cat $TMPDIR/qs.log
    echo "── pipewire.log:"; cat $TMPDIR/pipewire.log || true
    exit 1
  fi
  test "$(grep -c '^ok: ' $TMPDIR/result)" -eq 25
  touch $out
''

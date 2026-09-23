# The audio taps' lifecycle against a REAL PipeWire daemon: the shell runs
# under a headless cage compositor (as in desktop-smoke), PipeWire runs
# from a private config with one null sink published as the default, and
# `meters status` (the IPC behind `vogix desktop meters`) plus the process
# table are the observations. Proven here:
#
# - with nothing playing — PipeWire down or up — no tap is launched;
# - a playback stream starts both taps and the output VU, and they stop
#   when it ends;
# - a source reads off, idle or waiting only once its tap process is gone:
#   a tap that cannot exit yet reads stopping;
# - the shell's own taps and VU monitors never light the privacy cell's
#   microphone flag; another program's capture stream does, until it ends;
# - a tap that dies is relaunched;
# - without a default sink the taps wait, and start when one appears;
# - a hidden bar's taps, VU monitors and stat samplers stop, per edge, and
#   come back when it is shown;
# - a PipeWire restart stops the taps and brings them back;
# - launched as the unit launches it, quickshell writes no DEBUG records.
#
# Every step waits for an event: a status the shell reports, a process
# exit, the shell's client on the PipeWire daemon. None waits a fixed
# time, and the run's overall timeout is the only bound.
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

  # A daemon with no hardware (pipewire-daemon.nix): one null sink,
  # published as the default the way a session manager would.
  pipewireConf = import ./pipewire-daemon.nix { inherit pkgs; } "taps-pipewire.conf" ''
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
  '';

  inner = pkgs.writeShellScript "taps-inner" ''
    set -u
    export QS_NO_RELOAD_POPUP=1 QS_DISABLE_FILE_WATCHER=1 QT_QUICK_BACKEND=software
    # As the vogix-desktop unit sets it: a PipeWire that is not up yet is
    # waited for.
    export QS_PIPEWIRE_IMMEDIATE_RECONNECT=1
    out=$TMPDIR/result
    note() { echo "$*" >> "$out"; }
    # What the run waits for, and the last answer it had: reported if the
    # overall timeout ends the run.
    waiting() { echo "$*" > "$TMPDIR/awaiting"; }
    meters() { qs -p "$qml" ipc call meters status 2>&1; }
    # Polls `meters status` every 0.5 s until it contains $1; the status
    # that matched is left in $TMPDIR/matched.
    await() {
      local s
      waiting "meters: $1"
      while :; do
        s=$(meters)
        case "$s" in *"$1"*) echo "$s" > "$TMPDIR/matched"; note "ok: $1"; return 0 ;; esac
        echo "$s" > "$TMPDIR/last"
        sleep 0.5
      done
    }
    # Polls `privacy status` every 0.5 s until it is $1.
    await_privacy() {
      local s
      waiting "privacy: $1"
      while :; do
        s=$(qs -p "$qml" ipc call privacy status 2>&1)
        [ "$s" = "$1" ] && { note "ok: privacy $1"; return 0; }
        echo "$s" > "$TMPDIR/last"
        sleep 0.5
      done
    }
    # Polls the process table every 0.5 s until exactly one process named
    # $1 runs, other than $2 if given, and prints its PID. A tap takes its
    # name once bash has exec'd into it, and one that has exited but is
    # not reaped yet still counts.
    tap_pid() {
      local p
      waiting "one $1 other than ''${2:-none}"
      while :; do
        p=$(pgrep -x "$1")
        if [ -n "$p" ] && [ "$(echo "$p" | wc -l)" -eq 1 ] && [ "$p" != "''${2:-}" ]; then
          echo "$p"
          return 0
        fi
        echo "$p" > "$TMPDIR/last"
        sleep 0.5
      done
    }
    # The shell is a client of the running daemon, so the daemon is up and
    # the shell's connection to it too. Polls every 0.5 s.
    await_shell_on_pw() {
      waiting "the shell on PipeWire"
      until pw-cli ls Client 2>/dev/null | grep -qF "pipewire.sec.pid = \"$QSPID\""; do
        sleep 0.5
      done
      note "ok: the shell is on PipeWire"
    }
    # The tap processes, as the process table lists them (zombies too).
    taps() { ps -o pid=,stat=,comm= -C cava,pw-record; }
    # No tap process exists. Called once the shell reports its sources
    # off, idle or waiting, which it does only after quickshell has reaped
    # the taps: there is nothing left to wait for.
    notaps() {
      local left
      left=$(taps)
      if [ -n "$left" ]; then
        note "FAIL: a tap is running: $1:" $left
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
    running="spectrum:running scope:running vu-out:on"
    idle="spectrum:idle scope:idle vu-out:idle"

    # Launched as the unit launches it (desktop.detailedLogs = false).
    qs -p "$qml" --no-detailed-logs > "$TMPDIR/qs.log" 2>&1 &
    QSPID=$!

    # 1. Before PipeWire, and with PipeWire but nothing playing: idle.
    await "$idle vu-mic:on"
    notaps "before PipeWire"
    startpw
    await_shell_on_pw
    await "$idle vu-mic:on"
    notaps "nothing playing"

    # 2. Playback starts both taps and the output VU.
    play
    await "$running"

    # 2b. The taps (cava, the scope's pw-record) and the peak monitors
    # capture the output monitor, not a microphone: with all of them
    # running the privacy cell's microphone flag stays off. Another
    # program's capture stream turns it on, and its end turns it off.
    await_privacy "mic:off screencast:off"
    pw-cat --record --raw --format=s16 --rate=48000 --channels=2 /dev/null >> "$TMPDIR/pipewire.log" 2>&1 &
    CAPPID=$!
    await_privacy "mic:on screencast:off"
    kill $CAPPID
    wait $CAPPID
    await_privacy "mic:off screencast:off"
    cava1=$(tap_pid cava) pw1=$(tap_pid pw-record)

    # 3. A tap that dies comes back as a new process: once both have
    # exited, the shell starts each again.
    kill $cava1 $pw1
    waiting "cava $cava1 and pw-record $pw1 to exit"
    waitpid --exited $cava1 $pw1
    cava2=$(tap_pid cava "$cava1")
    pw2=$(tap_pid pw-record "$pw1")
    await "$running"
    note "pids: cava $cava1 -> $cava2, pw-record $pw1 -> $pw2"
    note "ok: relaunched"

    # 4. No default sink: the taps wait; one appears: they start.
    pw-metadata -n default -d 0 default.audio.sink >> "$TMPDIR/pipewire.log" 2>&1
    await "spectrum:waiting scope:waiting"
    notaps "no default sink"
    pw-metadata -n default 0 default.audio.sink '{ "name": "taps-sink" }' Spa:String:JSON >> "$TMPDIR/pipewire.log" 2>&1
    await "$running"

    # 5. Every source runs while its bar is shown; hiding an edge stops
    # exactly that edge's sources, hiding all stops everything, and
    # showing them again brings it all back.
    await "$running vu-mic:on stats:cpu,uptime"
    qs -p "$qml" ipc call bar hide bottom > /dev/null
    await "spectrum:running scope:off vu-out:off vu-mic:off stats:uptime"
    if pgrep -x pw-record > /dev/null; then
      note "FAIL: hiding the bottom bar left the scope tap:" $(taps)
    else
      tap_pid cava > /dev/null
      note "ok: bottom hidden"
    fi
    qs -p "$qml" ipc call bar hide all > /dev/null
    await "spectrum:off scope:off vu-out:off vu-mic:off stats:none"
    notaps "all bars hidden"
    qs -p "$qml" ipc call bar unhide all > /dev/null
    await "$running vu-mic:on stats:cpu,uptime"

    # 6. Playback ends: back to idle, no taps. The taps are frozen first
    # (SIGSTOP keeps the shell's SIGTERM pending), so they cannot exit:
    # once the shell has seen the playback end (the output VU is idle),
    # the spectrum and the scope read stopping, and idle only once their
    # taps are thawed and gone.
    cava6=$(tap_pid cava) pw6=$(tap_pid pw-record)
    kill -STOP $cava6 $pw6
    kill $PLAYPID
    wait $PLAYPID
    await "vu-out:idle"
    s=$(cat "$TMPDIR/matched")
    if [ "$(ps -o stat= -p "$cava6,$pw6" | cut -c1 | tr -d '\n')" != TT ]; then
      note "FAIL: the taps are not both frozen:" $(taps)
    else
      case "$s" in
        *"spectrum:stopping scope:stopping vu-out:idle"*) note "ok: stopping while the taps cannot exit" ;;
        *) note "FAIL: '$s' while both taps are frozen:" $(taps) ;;
      esac
    fi
    kill -CONT $cava6 $pw6
    await "$idle"
    notaps "playback ended"

    # 7. A PipeWire restart: everything stops with it and returns with it.
    play
    await "$running"
    kill $PWPID
    wait $PWPID $PLAYPID
    await "$idle"
    notaps "PipeWire stopped"
    startpw
    await_shell_on_pw
    play
    await "$running"

    kill $QSPID $PWPID $PLAYPID 2>/dev/null
    wait
    note done
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
    pkgs.util-linux
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

  # The run's one bound. A step whose event never comes holds the run
  # until here, and the result names it.
  WLR_BACKENDS=headless WLR_LIBINPUT_NO_DEVICES=1 WLR_RENDERER=pixman \
    timeout -k 10 600 cage -- ${inner} || true

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
  if grep -q '^FAIL' $TMPDIR/result || ! grep -qx done $TMPDIR/result; then
    grep -qx done $TMPDIR/result \
      || echo "── the run stopped waiting for $(cat $TMPDIR/awaiting) (last: $(cat $TMPDIR/last 2>/dev/null))"
    echo "── qs.log:"; cat $TMPDIR/qs.log
    echo "── pipewire.log:"; cat $TMPDIR/pipewire.log || true
    exit 1
  fi
  test "$(grep -c '^ok: ' $TMPDIR/result)" -eq 29
  touch $out
''

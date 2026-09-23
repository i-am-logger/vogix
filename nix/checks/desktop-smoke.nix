# The shell actually RUNS: a headless cage compositor hosts the real
# quickshell on the real QML, exactly as the vogix-desktop unit starts it
# (its ExecStart and Environment, runtime PATH wrapper included), on a
# private D-Bus session bus. The verbs are driven through the real
# `vogix desktop` CLI wherever one exists, so every call crosses the same
# CLI → IPC → shell path a keybinding takes.
#
# The layout is the shipped default (desktop-json.pin.json) plus every
# registry widget that default leaves out, so each widget the shell ships
# instantiates here, and custom cells covering every trigger but a click
# (the bars do not map under cage, so nothing can be clicked).
# Host tools the shell reads (hyprctl, systemctl, tailscale) are fixtures
# answering the way the real tools do, and a real MPRIS player (mpv)
# plays on the session bus. Every log the shell writes here passes a gate:
# no script error, binding problem, failed component, failed spawn or
# warning of the shell's own beyond the exact lines a fixture provokes.
#
# Compositor-, PipeWire- and NetworkManager-dependent behaviour is out of
# reach here (cage offers no layer-shell or session lock, and neither
# daemon runs): desktop-hyprland covers it on a real Hyprland session, and
# desktop-taps covers the taps against a real PipeWire. The geometry run is
# the exception: it feeds every widget the widest realistic data it shows
# (desktop-geometry-feed.nix), PipeWire and Hyprland's sockets included,
# so each placement is measured showing it.
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
  # Registry widgets the default layout does not place: on the top bar
  # each that renders horizontally, on the right rail each that renders
  # vertically (a widget the registry confines to neither sits on both).
  extras = orientation: builtins.filter (n: !(builtins.elem n placed)) registry.placeable.${orientation};
  # Every placement the registry permits, as the geometry probe names it:
  # "<edge> <widget>".
  placements = lib.concatMap
    (edge: map (n: "${edge} ${n}") registry.placeable.${registry.edgeOrientation edge})
    [ "top" "bottom" "left" "right" ];

  fixture = lib.recursiveUpdate pin {
    # One mount every host has and one no host has: the mounts cell gauges
    # the first unless the root is RAM-backed, and has no entry for the
    # second.
    meters.mounts = [ "/" "/vogix-smoke-absent" ];
    # Custom cells over every trigger but a click: first show (text, json,
    # a stream, a stream through a pipeline), a watched file's creation
    # and change, the IPC refresh, the timer (one cell due every 2 s, one
    # hourly), a parked bar's return and a reload that redefines a cell.
    # The top bar's pulse, due every 2 s like the ticker, is the clock a
    # parked rail is watched against. @RT@ becomes the runtime dir once
    # the file is in place.
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
      ticker = {
        command = "n=$(cat @RT@/smoke-tick 2>/dev/null || echo 0); n=$((n + 1)); echo $n > @RT@/smoke-tick; echo TICK-$n";
        interval = 2;
      };
      slow = {
        command = "n=$(cat @RT@/smoke-slow 2>/dev/null || echo 0); n=$((n + 1)); echo $n > @RT@/smoke-slow; echo SLOW-$n";
        interval = 3600;
      };
      pulse = {
        command = "n=$(cat @RT@/smoke-pulse 2>/dev/null || echo 0); n=$((n + 1)); echo $n > @RT@/smoke-pulse; echo PULSE-$n";
        interval = 2;
      };
      # A stream behind a pipeline, the shape of a `journalctl -f | grep`
      # producer: neither stage is the `sh` a stop signals.
      piped = { command = "tail -f @RT@/smoke-piped | grep --line-buffered PIPED-"; stream = true; };
      # Redefined by a reload: a new command for a cell on a parked rail,
      # and an interval for a live cell that had none.
      renamed = { command = "echo OLD-1"; };
      later = {
        command = "n=$(cat @RT@/smoke-later 2>/dev/null || echo 0); n=$((n + 1)); echo $n > @RT@/smoke-later; echo LATER-$n";
      };
    };
    bars = {
      top.layout.center = pin.bars.top.layout.center ++ extras "horizontal"
        ++ [ "custom/smoke" "custom/gauge" "custom/pulse" "custom/later" ];
      right.layout.center = pin.bars.right.layout.center ++ extras "vertical"
        ++ [ "custom/watched" "custom/counter" "custom/stream" "custom/ticker" "custom/slow" "custom/piped" "custom/renamed" ];
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

  # The input engine's mode table (input.json) for the mode cell; the state
  # run writes the current mode beside it.
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
  # The geometry run's data: every widget fed the widest realistic
  # content it shows.
  geometryFeed = import ./desktop-geometry-feed.nix { inherit pkgs; };
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
    pkgs.procps
    qsPkgs.quickshell
    qsPkgs.vogix
    hyprctlFixture
    systemctlFixture
    tailscaleFixture
  ] ++ geometryFeed.packages;
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
  # Every step waits for an event: an answer from the shell, a state a
  # verb reports, a file's content, a process's exit. None waits a fixed
  # time: the run's overall timeout is the only bound, and the result
  # names what a run it stopped was waiting for.
  waiting() { echo "$*" > $TMPDIR/awaiting; }
  # Runs its command every 0.1 s until it succeeds.
  await() {
    waiting "$1"
    shift
    until "$@" > $TMPDIR/last 2>&1; do sleep 0.1; done
  }
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
  # A verb, asked every 0.1 s until its reply matches the extended regex
  # EXPECTED; LABEL prefixes the reply that did.
  vuntil() {
    local label=$1 expected=$2 reply
    shift 2
    waiting "vogix desktop $* to answer /$expected/"
    while :; do
      reply=$(vogix desktop "$@" 2>&1)
      [[ $reply =~ $expected ]] && break
      printf '%s\n' "$reply" > $TMPDIR/last
      sleep 0.1
    done
    printf '%s\n' "$reply" | sed "s|^|$label |" >> $R
  }
  # A verb, asked every 0.1 s while its whole reply matches the extended
  # regex BEFORE (an earlier state); LABEL prefixes the first reply that
  # does not.
  vnext() {
    local label=$1 before=$2 reply
    shift 2
    waiting "vogix desktop $* to answer other than /$before/"
    while :; do
      reply=$(vogix desktop "$@" 2>&1)
      [[ $reply =~ ^($before)$ ]] || break
      sleep 0.1
    done
    printf '%s\n' "$reply" | sed "s|^|$label |" >> $R
  }
  # A transport-only IPC function (no verb reaches it).
  ipc() {
    label=$1
    shift
    qs -c vogix ipc call "$@" 2>&1 | sed "s|^|$label |" >> $R
  }
  # The same, asked every 0.1 s until it answers ANSWER.
  ipcuntil() {
    local label=$1 answer=$2 reply
    shift 2
    waiting "qs ipc call $* to answer $answer"
    while :; do
      reply=$(qs -c vogix ipc call "$@" 2>&1)
      [ "$reply" = "$answer" ] && break
      printf '%s\n' "$reply" > $TMPDIR/last
      sleep 0.1
    done
    echo "$label $reply" >> $R
  }
  # How many times a custom cell's command has run: the count it keeps in
  # its file (0 before the first run).
  count() { cat $XDG_RUNTIME_DIR/smoke-$1 2>/dev/null || echo 0; }
  # No custom command whose command line matches the regex is running.
  none_running() { ! pgrep -f "$1" > /dev/null; }

  # An MPRIS player on the session bus before the shell starts: the
  # media cell comes up with a player to show and control.
  mpv --no-config --really-quiet --idle=no --loop=inf --ao=null --vo=null \
    --script=${pkgs.mpvScripts.mpris}/share/mpv/scripts/mpris.so \
    av://lavfi:sine=frequency=440 > $TMPDIR/mpv.log 2>&1 &
  MPVPID=$!
  await "the player to play" sh -c '[ "$(playerctl status)" = Playing ]'
  # The line the piped cell's stream shows.
  echo PIPED-1 > $XDG_RUNTIME_DIR/smoke-piped
  launch qs.log
  # The shell answers with the bar state once it has read desktop.json.
  vuntil status 'bar: top:' status
  kill -0 $QSPID 2>/dev/null && echo ALIVE >> $R
  # Each custom cell's first run, on first show, lands before any bar
  # hides: hiding a bar cuts its cells' runs short and they run again,
  # which would move the counts the counter and the hourly cell are
  # checked by. (The 2 s ticker and pulse are counted only from a known
  # point.)
  for cell in smoke gauge watched renamed later; do
    vnext custom-$cell 'inactive|pending' custom status $cell
  done
  vuntil custom-stream '^S-2$' custom status stream
  vuntil custom-piped '^PIPED-1$' custom status piped
  for cell in counter:RUN-1 slow:SLOW-1; do
    await "custom/''${cell%:*}'s first run" sh -c "[ \"\$(vogix desktop custom status ''${cell%:*})\" = ''${cell#*:} ]"
  done
  v bar-hide-left bar hide left
  v bar-show bar show
  v bar-toggle bar toggle
  v bar-toggle-back bar toggle
  # A bar nobody can see samples nothing; shown again, it samples.
  v bars-hidden bar hide
  vuntil meters-hidden '^spectrum:off scope:off vu-out:off vu-mic:off stats:none$' meters
  v bars-shown bar show
  vuntil meters-shown ' stats:cpu,memory,net,disk,gpu,uptime,temp,fans,mounts$' meters
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
  # The shown bars' samplers have delivered: memory, CPU and the df table.
  await "the stats' first samples" sh -c 'vogix desktop stats > $TMPDIR/stats.json 2>&1 &&
    jq -e ".memory != null and .cpu != null and (.mounts | length) > 0" $TMPDIR/stats.json'
  v privacy privacy
  v custom-undefined custom status undefined
  echo W-1 > $XDG_RUNTIME_DIR/smoke-watch
  vnext custom-watched-created 'failed: exited 1' custom status watched
  echo W-2 > $XDG_RUNTIME_DIR/smoke-watch
  vnext custom-watched-changed W-1 custom status watched
  v custom-counter-refresh custom refresh counter
  vnext custom-counter RUN-1 custom status counter
  # Custom cells run only while their bar is on screen. The 2 s ticker
  # has ticked; with the right rail parked, nothing on it runs, and a
  # watched change and a refresh wait. Back on screen, the ticker (its
  # result older than its interval), the watched cell and the refreshed
  # counter run at once, and the hourly cell does not.
  await "the 2 s ticker to run twice" sh -c "[ \$(cat $XDG_RUNTIME_DIR/smoke-tick 2>/dev/null || echo 0) -ge 2 ]"
  v park-right bar hide right
  # A run the parking cut short has exited, and so has every process it
  # started: both stages of the piped cell's pipeline, from this hide and
  # from every earlier one.
  await "the parked rail's commands to exit" none_running 'smoke-(tick|count|slow|watch|piped)|PIPED-'
  t0=$(count tick)
  s0=$(count slow)
  echo W-3 > $XDG_RUNTIME_DIR/smoke-watch
  v custom-parked-refresh custom refresh counter
  # Parked while the top bar's 2 s pulse runs twice: the shell's timers
  # ran for a whole interval, in which the parked ticker was due.
  p0=$(count pulse)
  await "the pulse to run twice" sh -c "[ \$(cat $XDG_RUNTIME_DIR/smoke-pulse) -ge $((p0 + 2)) ]"
  echo "custom-parked-ticks $t0 $(count tick)" >> $R
  v custom-parked-watched custom status watched
  v custom-parked-counter custom status counter
  t1=$(count tick)
  v unpark-right bar show right
  # The ticker's run on return, counted by the same read that sees it: a
  # second run would come an interval later.
  await "the ticker to run on return" sh -c "n=\$(cat $XDG_RUNTIME_DIR/smoke-tick); [ \$n -gt $t1 ] && echo \$n > $TMPDIR/tick-seen"
  echo "custom-unparked-ticks $(($(cat $TMPDIR/tick-seen) - t1))" >> $R
  vnext custom-unparked-watched W-2 custom status watched
  vnext custom-unparked-counter RUN-2 custom status counter
  # Had the hourly cell started with the others, its run would still be
  # going or would have counted.
  await "no hourly run in flight" none_running smoke-slow
  echo "custom-unparked-slow $s0 $(count slow)" >> $R
  # Back on screen, the piped cell's stream runs as one pipeline.
  piped_tail="tail -f $XDG_RUNTIME_DIR/smoke-piped"
  await "the piped cell's pipeline back on screen" pgrep -xf "$piped_tail"
  echo "custom-piped-pipelines $(pgrep -cxf "$piped_tail")" >> $R
  # A reload that removes a cell ends every process its command started.
  jq 'del(.custom.piped) | .bars.right.layout.center -= ["custom/piped"]' \
    $XDG_STATE_HOME/vogix/desktop.json > $TMPDIR/unpiped.json
  mv $TMPDIR/unpiped.json $XDG_STATE_HOME/vogix/desktop.json
  v reload-unpiped reload
  ipcuntil custom-piped-removed 'unknown custom cell: piped' custom status piped
  await "the removed cell's pipeline to exit" none_running 'smoke-piped|PIPED-'
  # A reload that redefines cells: a new command for a cell on the parked
  # rail, which runs once the rail is back, and an interval for a live
  # cell that had none, which runs it again.
  v park-right-redefine bar hide right
  jq '.custom.renamed.command = "echo NEW-1" | .custom.later.interval = 1' \
    $XDG_STATE_HOME/vogix/desktop.json > $TMPDIR/redefined.json
  mv $TMPDIR/redefined.json $XDG_STATE_HOME/vogix/desktop.json
  v reload-redefined reload
  await "the new interval to run custom/later again" sh -c "[ \$(cat $XDG_RUNTIME_DIR/smoke-later) -ge 2 ]"
  v custom-redefined-parked custom status renamed
  v unpark-right-redefine bar show right
  vuntil custom-redefined '^NEW-1$' custom status renamed
  vnext keyboard 'device:- layouts:- active:- caps:unknown' keyboard
  # The input engine's lock document, handled as the engine does:
  # written tmp + rename, rewritten the same way, removed on stop. The
  # LANG cell must follow every step through its file watch.
  put_locks() {
    printf '%s' "$1" > $XDG_STATE_HOME/vogix/input-locks.json.tmp
    mv $XDG_STATE_HOME/vogix/input-locks.json.tmp $XDG_STATE_HOME/vogix/input-locks.json
  }
  put_locks '{"capsLock":true,"numLock":false,"scrollLock":null}'
  vuntil locks-on ' caps:on$' keyboard
  put_locks '{"capsLock":false,"numLock":false,"scrollLock":null}'
  vuntil locks-off ' caps:off$' keyboard
  put_locks '{"capsLock":null,"numLock":null,"scrollLock":null}'
  vuntil locks-null ' caps:unknown$' keyboard
  put_locks '{"capsLock":true,"numLock":false,"scrollLock":null}'
  vuntil locks-back ' caps:on$' keyboard
  rm $XDG_STATE_HOME/vogix/input-locks.json
  vuntil locks-gone ' caps:unknown$' keyboard
  # Two notifications, a critical one whose body wraps: cards in the
  # popup column, and the live set mirrored to the state file.
  notify-send -u critical -a smoke-critical "Critical summary" \
    "a body long enough to wrap across more than one line of the card, which exercises the wrapped text height path"
  notify-send -a smoke-normal -t 60000 "Normal summary" "short body"
  await "both notifications in the state file" \
    jq -e 'length == 2' $XDG_STATE_HOME/vogix/desktop/notifications.json
  ipc notify-count notify count
  cp $XDG_STATE_HOME/vogix/desktop/notifications.json $TMPDIR/notifications-1.json
  stop
  kill $MPVPID 2>/dev/null

  # Restart run: the live notifications come back (the state run reads
  # the times the restored cards carry).
  launch qs-restart.log
  ipcuntil restart-notify-count 2 notify count
  stop

  # Scanlines run: the texture on the bars and the restored cards.
  cp $TMPDIR/scanlines.json $XDG_STATE_HOME/vogix/desktop.json
  launch qs-scanlines.log
  ipcuntil scanlines-notify-count 2 notify count
  kill -0 $QSPID 2>/dev/null && echo SCANLINES-ALIVE >> $R
  stop
  # Geometry run, on the shipped default desktop.json's bar and font
  # sizes, with every data source fed (desktop-geometry-feed.nix) and a
  # player on the bus, so every widget shows and is measured: the probe
  # exits on its own with its verdict.
  install -m 644 $pinJson $XDG_STATE_HOME/vogix/desktop.json
  mpv --no-config --really-quiet --idle=no --loop=inf --ao=null --vo=null \
    --script=${pkgs.mpvScripts.mpris}/share/mpv/scripts/mpris.so \
    av://lavfi:sine=frequency=440 > $TMPDIR/mpv-geometry.log 2>&1 &
  MPVPID=$!
  await "the geometry run's player to play" sh -c '[ "$(playerctl status)" = Playing ]'
  . ${geometryFeed.script}
  feed_start && echo FEED-OK >> $R
  waiting "the geometry probe to exit"
  feed_run ${desktopEnv} qs -p $TMPDIR/geometry > $TMPDIR/qs-geometry.log 2>&1
  echo "GEOMETRY-EXIT $?" >> $R
  feed_stop
  kill $MPVPID 2>/dev/null

  # State run, with the scanline texture on: the window title, the mode
  # cell's table and then its loss, the restored cards' texture and
  # arrival times, and the media cell following a player and sending its
  # transport to the player that last played.
  cp $TMPDIR/scanlines.json $XDG_STATE_HOME/vogix/desktop.json
  cp $inputJsonPath $XDG_STATE_HOME/vogix/input.json
  echo normal > $XDG_STATE_HOME/vogix/current-mode
  # Two players: a paused one the shell sees first, then one playing.
  mpv --no-config --really-quiet --idle=no --loop=inf --ao=null --vo=null --pause \
    --script=${pkgs.mpvScripts.mpris}/share/mpv/scripts/mpris.so \
    av://lavfi:sine=frequency=440 > $TMPDIR/mpv-state-first.log 2>&1 &
  MPVPID=$!
  await "the state run's first player, paused" sh -c '[ "$(playerctl status)" = Paused ]'
  ${desktopEnv} qs -p $TMPDIR/state > $TMPDIR/qs-state.log 2>&1 &
  STATEPID=$!
  await "the state probe to see the first player" grep -q 'STATE media-first ' $TMPDIR/qs-state.log
  mpv --no-config --really-quiet --idle=no --loop=inf --ao=null --vo=null \
    --script=${pkgs.mpvScripts.mpris}/share/mpv/scripts/mpris.so \
    av://lavfi:sine=frequency=440 > $TMPDIR/mpv-state.log 2>&1 &
  MPV2PID=$!
  waiting "the state probe to exit"
  wait $STATEPID
  echo "STATE-EXIT $?" >> $R
  kill $MPVPID $MPV2PID 2>/dev/null

  # Rejection run: a schema-1 desktop.json is refused, loudly, without
  # taking the shell down.
  cp $schema1JsonPath $XDG_STATE_HOME/vogix/desktop.json
  launch qs-schema1.log
  await "the schema-1 refusal" grep -q 'desktop.json schema 1 is not supported' $TMPDIR/qs-schema1.log
  vuntil schema1 '^top:' bar status
  kill -0 $QSPID 2>/dev/null && echo SCHEMA1-ALIVE >> $R
  stop
  echo DONE >> $R
  INNER
  chmod +x inner.sh
  # The run's one bound. A step whose event never comes holds the run
  # until here, and the result names it.
  WLR_BACKENDS=headless WLR_LIBINPUT_NO_DEVICES=1 WLR_RENDERER=pixman \
    timeout -k 10 900 cage -- dbus-run-session --config-file=${pkgs.dbus}/share/dbus-1/session.conf -- ${pkgs.runtimeShell} ./inner.sh || true

  echo "── result:"; cat $TMPDIR/result || true
  echo "── stats:"; cat $TMPDIR/stats.json || true
  echo "── geometry:"; grep -hE "GEOMETRY|FOOTPRINT|FIT" $TMPDIR/qs-geometry.log || true
  echo "── state:"; grep -h 'STATE' $TMPDIR/qs-state.log || true
  echo "── Hyprland requests:"; cat $TMPDIR/feed/hypr-requests.log || true
  echo "── notification arrival times:"; jq -c '[.[].at]' $TMPDIR/notifications-1.json || true
  grep -qx DONE $TMPDIR/result || {
    echo "── the run stopped waiting for $(cat $TMPDIR/awaiting) (last: $(cat $TMPDIR/last 2>/dev/null))"
    for log in qs-geometry.log qs-state.log; do
      grep -h 'waiting for' $TMPDIR/$log 2>/dev/null | tail -n 1
    done
    exit 1
  }

  # The log gate. A failure is a script error, a binding problem, a
  # component that did not load, a program the shell could not start, or
  # a warning the shell logged itself or quickshell's Hyprland IPC logged
  # (a request Hyprland's socket did not answer); only the exact lines a
  # fixture provokes are allowed, per log.
  failures='TypeError|ReferenceError|SyntaxError|RangeError|Binding loop|Unable to assign|Cannot assign|is not a type|is not installed|Failed to load configuration|Type [^ ]+ unavailable|Script [^ ]+ unavailable|\]: File not found|Process failed to start|^(WARN|ERROR|CRIT) +qml:|^(WARN|ERROR|CRIT) +quickshell\.hyprland\.ipc:'
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
  r 'status shell: running'
  r 'status bar: top:shown bottom:shown left:shown right:shown'
  r 'bar-hide-left top:shown bottom:shown left:hidden right:shown'
  r 'bar-show top:shown bottom:shown left:shown right:shown'
  r 'bar-toggle top:hidden bottom:hidden left:hidden right:hidden'
  r 'bar-toggle-back top:shown bottom:shown left:shown right:shown'
  r 'meters-hidden spectrum:off scope:off vu-out:off vu-mic:off stats:none'
  grep -q '^meters-shown .* stats:cpu,memory,net,disk,gpu,uptime,temp,fans,mounts$' $TMPDIR/result
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
  r 'privacy mic:off screencast:off'
  r 'custom-smoke SMOKE-42'
  r 'custom-gauge J-7'
  r 'custom-stream S-2'
  r 'custom-watched failed: exited 1'
  r 'custom-watched-created W-1'
  r 'custom-watched-changed W-2'
  r 'custom-counter-refresh refreshing'
  r 'custom-counter RUN-2'
  # Parked: the tick count held, the watched cell kept its value and the
  # refresh was queued. Back on screen: one tick at once, no hourly run,
  # and the watched change and the refresh taken.
  set -- $(grep '^custom-parked-ticks ' $TMPDIR/result)
  test "$2" = "$3" || { echo "a custom cell ran while its bar was parked: ticks $2 -> $3"; exit 1; }
  r 'custom-parked-refresh queued: custom/counter runs once its bar is on screen'
  r 'custom-parked-watched W-2'
  r 'custom-parked-counter RUN-2'
  r 'custom-unparked-ticks 1'
  set -- $(grep '^custom-unparked-slow ' $TMPDIR/result)
  test "$2" = "$3" || { echo "the hourly cell ran on coming back on screen: $2 -> $3"; exit 1; }
  r 'custom-unparked-watched W-3'
  r 'custom-unparked-counter RUN-3'
  # The piped stream: shown, one pipeline after the hides, and gone with
  # its cell.
  r 'custom-piped PIPED-1'
  r 'custom-piped-pipelines 1'
  r 'custom-piped-removed unknown custom cell: piped'
  # Redefined by a reload: the parked cell keeps its result until its rail
  # is back, then runs its new command.
  r 'custom-renamed OLD-1'
  r 'custom-later LATER-1'
  r 'custom-redefined-parked OLD-1'
  r 'custom-redefined NEW-1'
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
  # same two restored after a restart, and with the scanline texture on.
  r 'notify-count 2'
  jq -e 'length == 2 and all(.[]; (.at | type) == "number")' $TMPDIR/notifications-1.json
  r 'restart-notify-count 2'
  r SCANLINES-ALIVE
  r 'scanlines-notify-count 2'
  # A canvas instrument never lays out collapsed nor resizes when audio
  # arrives, and every placement the registry permits shows its fed data
  # and fits its bar: the probe's verdict, with a FIT line measuring each
  # of those placements at a size (neither side 0), the oscilloscope
  # measured and the media cell shown. Hyprland's request socket was
  # asked for exactly what quickshell asks at startup, and answered all
  # of it.
  r FEED-OK
  r 'GEOMETRY-EXIT 0'
  for placement in ${lib.escapeShellArgs placements}; do
    grep -qE "FIT $placement [1-9][0-9.]*x[1-9][0-9.]* in " $TMPDIR/qs-geometry.log \
      || { echo "no FIT line measuring $placement at a size:" $(grep "FIT $placement " $TMPDIR/qs-geometry.log); exit 1; }
  done
  test "$(sort -u $TMPDIR/feed/hypr-requests.log | tr '\n' ' ')" = 'j/clients j/monitors j/status j/workspaces ' \
    || { echo "Hyprland's request socket was asked:" $(sort -u $TMPDIR/feed/hypr-requests.log); exit 1; }
  grep -q 'GEOMETRY bottom oscilloscope [1-9][0-9.]*x[1-9][0-9.]*$' $TMPDIR/qs-geometry.log
  grep -q 'FIT bottom media [0-9]' $TMPDIR/qs-geometry.log
  # The window title from hyprctl; the mode label from input.json, then
  # the bare mode once input.json cannot be read; the texture on both
  # restored cards, and the times they arrived at in the first run; the
  # media cell playing, paused and playing again with its player, and
  # its play/pause resuming that player, not the first.
  r 'STATE-EXIT 0'
  grep -q 'STATE window-title smoke window title$' $TMPDIR/qs-state.log
  grep -q 'STATE mode-label NRM-SMOKE$' $TMPDIR/qs-state.log
  grep -q 'STATE mode-label-after-failure normal$' $TMPDIR/qs-state.log
  grep -q 'STATE card-scanlines 2/2$' $TMPDIR/qs-state.log
  times=$(jq -r 'map(.at | tostring) | join(",")' $TMPDIR/notifications-1.json)
  grep -q "STATE card-times $times\$" $TMPDIR/qs-state.log \
    || { echo "the restored cards carry" $(grep -o 'STATE card-times .*' $TMPDIR/qs-state.log) "; they arrived at $times"; exit 1; }
  grep -q 'STATE media-first paused$' $TMPDIR/qs-state.log
  grep -q 'STATE media-playing true$' $TMPDIR/qs-state.log
  grep -q 'STATE media-paused false$' $TMPDIR/qs-state.log
  grep -q 'STATE media-resumed true$' $TMPDIR/qs-state.log
  grep -q 'STATE media-transport the player last playing$' $TMPDIR/qs-state.log \
    || { echo "the media cell's play/pause resumed" $(grep -o 'STATE media-transport .*' $TMPDIR/qs-state.log); exit 1; }
  r SCHEMA1-ALIVE
  r 'schema1 top:off bottom:off left:off right:off'
  grep -q 'desktop.json schema 1 is not supported' $TMPDIR/qs-schema1.log
  touch $out
''

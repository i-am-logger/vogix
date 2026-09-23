# How the desktop checks end what they run under cage, as shell functions
# for their scripts to source.
#
# A run of the shell ends as the vogix-desktop unit's does: systemd
# signals the unit's whole control group (KillMode=control-group), so
# nothing the shell started outlives it, a command in a session of its
# own or a detached one included. quickshell has no SIGTERM handler, so
# SIGTERM to its pid alone leaves every process it started running. A
# check starts each run with VOGIX_SHELL_RUN=<its name> in the shell's
# environment; every process the run starts inherits it, and the
# processes carrying it are that run's control group.
#
# cage takes its client to have exited only once no process holds the
# write end of the pipe it watches for that (cage.c,
# spawn_primary_client), and every process a check starts under cage
# inherits that end: a process a check leaves running holds cage until
# the check's bound. cage_verdict names it.
{ pkgs }:
pkgs.writeText "shell-runs.sh" ''
  # The processes of the run NAME still running, one per line: pid and
  # command line.
  run_members() {
    local f
    for f in $(grep -lsxzF "VOGIX_SHELL_RUN=$1" /proc/[0-9]*/environ); do
      f=''${f%/environ}
      printf '%s %s\n' "''${f#/proc/}" "$(tr '\0' ' ' 2>/dev/null < $f/cmdline)"
    done
  }

  # Succeeds once no process of the run NAME is left; lists those that
  # are.
  run_ended() {
    local left
    left=$(run_members "$1")
    [ -z "$left" ] || { printf '%s\n' "$left"; return 1; }
  }

  # Ends the run NAME as systemd stops the unit: SIGTERM to every process
  # of it at once, then a wait, polled every 0.1 s, for each to exit. The
  # step is named through the caller's `waiting`, and the processes still
  # there are left in $TMPDIR/last.
  end_run() {
    local pids
    pids=$(run_members "$1" | cut -d' ' -f1)
    [ -z "$pids" ] || kill $pids 2>/dev/null
    waiting "every process of the $1 run to exit"
    until run_ended "$1" > $TMPDIR/last 2>&1; do sleep 0.1; done
  }

  # Every process running but this shell, one per line: pid and command
  # line.
  left_running() {
    local p cmd
    for p in /proc/[0-9]*; do
      p=''${p#/proc/}
      [ "$p" = $$ ] && continue
      cmd=$(tr '\0' ' ' 2>/dev/null < /proc/$p/cmdline) || continue
      [ -z "$cmd" ] || echo "$p $cmd"
    done
  }

  # How a run that stopped before it was done ended, from cage's STATUS
  # under a bound of BOUND s: at the bound, on the step the caller's
  # `waiting` last named, or with a status of its own after that step.
  # The step's last observation is $TMPDIR/last.
  stopped_short() {
    local step
    step="$(cat $TMPDIR/awaiting 2>/dev/null || true) (last: $(cat $TMPDIR/last 2>/dev/null || true))"
    case $1 in
      124 | 137) echo "── the run stopped at the $2 s bound, waiting for $step" ;;
      *) echo "── the run ended with cage's status $1 before it was done; its last wait was for $step" ;;
    esac
  }

  # The verdict on cage's STATUS once the run is done, under a bound of
  # BOUND s: 0 passes; 124, or 137 once killed, is the bound, which cage
  # reached held by what the run left running (listed); any other status
  # is the run's own.
  cage_verdict() {
    case $1 in
      0) return 0 ;;
      124 | 137)
        echo "── the run was done, but cage ran on to the $2 s bound, held by what the run left running:"
        left_running
        ;;
      *) echo "── cage exited with status $1" ;;
    esac
    return 1
  }
''

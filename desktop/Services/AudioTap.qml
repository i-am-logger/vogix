// A PipeWire capture subprocess (cava, pw-record) kept alive for as long
// as it is wanted. The command execs into the tap, so the process
// quickshell holds IS the tap: its exit is the tap's death, and a stop
// ends it with no pipeline stage left behind. A stop is SIGTERM, and
// SIGKILL once killAfterMs has passed without the tap exiting.
//
// Every tap captures the default sink's monitor, so it runs only while
// quickshell's own PipeWire connection is ready and a default sink
// exists: a login race or a PipeWire restart leaves it waiting, and the
// readiness edge starts it. An exit nobody asked for relaunches after
// 1, 2, 4, 8 and 16 s; a run that lasted stableMs clears the count. The
// sixth quick failure parks the tap until PipeWire reconnects, the
// default sink changes or its widgets come back, so a tap that cannot
// run costs five relaunches, not a loop.
//
// A command that changes while the tap runs (a reload that changes the
// tap's configuration) restarts it on the new command.
import QtQuick
import Quickshell
import Quickshell.Io
import Quickshell.Services.Pipewire

Scope {
    id: root

    // The name the log and `vogix desktop meters` use.
    required property string name
    required property list<string> command
    // A visible consumer wants this tap's data.
    property bool wanted: false

    // One line of the tap's stdout.
    signal line(string data)

    readonly property bool canRun: root.wanted && Pipewire.ready && Pipewire.defaultAudioSink !== null
    readonly property bool running: proc.running
    readonly property int maxRelaunches: 5
    readonly property int stableMs: 30000
    // cava acts on SIGTERM only in its PipeWire stream's process callback
    // (0.10.7, input/pipewire.c): while its stream is streaming, a graph
    // that delivers no buffer holds it in the stop, its main thread joined
    // on the audio thread. A tap that acts on SIGTERM exits in about
    // 0.1 s; 2 s is twenty times that, and leaves 3 of the 5 s within
    // which a locked session samples nothing.
    readonly property int killAfterMs: 2000
    readonly property int _sigkill: 9

    // Consecutive exits, each within stableMs of its start.
    property int failures: 0
    property bool parked: false

    property bool _stopping: false
    property real _startedAt: 0
    // The last line the tap wrote to stderr, quoted when it exits.
    property string _lastError: ""

    // running · stopping: asked to stop, not exited yet (killed once
    // killAfterMs has passed) · off: nothing
    // wants it · waiting: wanted, PipeWire or a default sink is missing ·
    // retrying: a relaunch is scheduled · parked. The process counts until
    // quickshell has reaped it, so every state but the first two means no
    // tap process exists.
    function status(): string {
        if (proc.running)
            return root._stopping ? "stopping" : "running";
        if (root.parked)
            return "parked";
        if (retry.running)
            return "retrying";
        return root.wanted ? "waiting" : "off";
    }

    // The one place the process is started or stopped.
    function _sync(): void {
        if (root.canRun && !root.parked) {
            // Setting running on a live Process arms quickshell's own
            // restart-on-exit, which would bypass the backoff below.
            if (!proc.running && !retry.running) {
                root._startedAt = Date.now();
                root._lastError = "";
                proc.running = true;
            }
            return;
        }
        retry.stop();
        root._stop();
    }

    // SIGTERM, and the deadline for the exit.
    function _stop(): void {
        if (!proc.running || root._stopping)
            return;
        root._stopping = true;
        proc.running = false;
        deadline.restart();
    }

    // A fresh start on every readiness or demand edge: PipeWire came
    // back, a default sink appeared, a widget became visible.
    function _reset(): void {
        root.failures = 0;
        root.parked = false;
        retry.stop();
        root._sync();
    }

    onCanRunChanged: root._reset()

    Connections {
        target: Pipewire

        // A different default sink is a new chance for a tap that is
        // parked or backing off; a running tap follows the default on its
        // own (stream.capture.sink).
        function onDefaultAudioSinkChanged(): void {
            if (!proc.running && (root.parked || retry.running))
                root._reset();
        }
    }

    Process {
        id: proc

        command: root.command

        // quickshell starts a new command only on the next start, and
        // emits this only when the list differs. The stop's exit runs
        // _sync, which starts the new command.
        onCommandChanged: root._stop()

        stdout: SplitParser {
            onRead: data => root.line(data)
        }

        stderr: SplitParser {
            onRead: data => root._lastError = data
        }

        onRunningChanged: {
            if (running)
                return;
            deadline.stop();
            if (root._stopping) {
                root._stopping = false;
                // Demand may have come back while it was stopping.
                root._sync();
                return;
            }
            // Died with its PipeWire connection: the ready edge restarts it.
            if (!root.canRun)
                return;
            if (Date.now() - root._startedAt >= root.stableMs)
                root.failures = 0;
            root.failures++;
            const why = root._lastError === "" ? "" : " (" + root._lastError + ")";
            if (root.failures > root.maxRelaunches) {
                root.parked = true;
                console.warn("vogix: " + root.name + " exited" + why + " after " + root.maxRelaunches
                    + " relaunches; parked until PipeWire, the default sink or its widgets change");
                return;
            }
            retry.interval = 1000 * Math.pow(2, root.failures - 1);
            console.warn("vogix: " + root.name + " exited" + why + "; relaunching in "
                + retry.interval / 1000 + " s");
            retry.restart();
        }
    }

    Timer {
        id: retry
        repeat: false
        onTriggered: root._sync()
    }

    // The stop's deadline: armed with its SIGTERM, cancelled by the exit.
    Timer {
        id: deadline
        interval: root.killAfterMs
        repeat: false
        onTriggered: {
            console.warn("vogix: " + root.name + " did not exit within " + root.killAfterMs / 1000
                + " s of SIGTERM; killing it");
            proc.signal(root._sigkill);
        }
    }
}

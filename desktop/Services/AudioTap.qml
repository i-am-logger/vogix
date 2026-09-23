// A PipeWire capture subprocess (cava, pw-record) kept alive for as long
// as it is wanted. The command execs into the tap, so the process
// quickshell holds IS the tap: its exit is the tap's death and SIGTERM
// stops it, with no pipeline stage left behind.
//
// Every tap captures the default sink's monitor, so it runs only while
// quickshell's own PipeWire connection is ready and a default sink
// exists: a login race or a PipeWire restart leaves it waiting, and the
// readiness edge starts it. An exit nobody asked for relaunches after
// 1, 2, 4, 8 and 16 s; a run that lasted stableMs clears the count. The
// sixth quick failure parks the tap until PipeWire reconnects, the
// default sink changes or its widgets come back, so a tap that cannot
// run costs five relaunches, not a loop.
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

    // Consecutive exits, each within stableMs of its start.
    property int failures: 0
    property bool parked: false

    property bool _stopping: false
    property real _startedAt: 0
    // The last line the tap wrote to stderr, quoted when it exits.
    property string _lastError: ""

    // running · stopping: asked to stop, not exited yet · off: nothing
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
        if (proc.running && !root._stopping) {
            root._stopping = true;
            proc.running = false;
        }
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

        stdout: SplitParser {
            onRead: data => root.line(data)
        }

        stderr: SplitParser {
            onRead: data => root._lastError = data
        }

        onRunningChanged: {
            if (running)
                return;
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
}

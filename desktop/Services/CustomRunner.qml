pragma ComponentBehavior: Bound
// One custom cell's command (desktop.json `custom.<name>`), shared by every
// cell that shows it and live only while one does. A run starts when the
// first cell appears, every `interval` seconds, when a `watch` file
// changes, after the cell's click action finishes, and on `vogix desktop
// custom refresh`. A trigger during a one-shot run queues exactly one
// re-run, so a burst of file changes costs one more run, never a pile-up.
// A `stream` command stays up and publishes every line it prints; a
// trigger relaunches it only once it has exited.
import QtQuick
import Quickshell
import Quickshell.Io

Scope {
    id: root

    required property string name
    property var def: ({})
    property bool active: false

    readonly property string command: def.command ?? ""
    readonly property bool stream: def.stream ?? false
    readonly property bool json: (def.output ?? "text") === "json"
    readonly property int intervalSec: def.interval ?? 0
    readonly property list<string> watch: def.watch ?? []
    readonly property string clickCommand: def.onClick ?? ""

    // What the cell shows. `level` colors the value: normal, warning or
    // danger (json output sets it); `meter` is json's 0..1 gauge, -1 for
    // none.
    property string text: ""
    property string level: "normal"
    property real meter: -1
    // Why the latest run failed — a non-zero exit, a kill, or output that
    // does not read — and "" once one succeeds.
    property string failure: ""
    property bool hasResult: false

    // One line for `vogix desktop custom status`.
    readonly property string status: !active ? "inactive"
        : failure !== "" ? "failed: " + failure
        : hasResult ? text
        : "pending"

    property bool _queued: false
    property bool _stopping: false
    // A one-shot run's output, published once the run exits: its first
    // non-empty line (text), or the whole of it up to _jsonCap (json).
    property string _firstLine: ""
    property string _all: ""
    readonly property int _jsonCap: 64 * 1024

    function trigger(): void {
        if (!root.active || root.command === "")
            return;
        if (proc.running) {
            if (!root.stream)
                root._queued = true;
            return;
        }
        root._firstLine = "";
        root._all = "";
        proc.running = true;
    }

    // A changed command or a hidden cell ends the current run without
    // counting it as a failure; a changed command starts the new one.
    function _stop(thenRun: bool): void {
        root._queued = thenRun;
        if (proc.running) {
            root._stopping = true;
            proc.running = false;
        } else if (thenRun) {
            root._queued = false;
            root.trigger();
        }
    }

    function click(): void {
        if (root.clickCommand === "") {
            root.trigger();
            return;
        }
        if (!clickProc.running)
            clickProc.running = true;
    }

    function _publish(raw: string): void {
        if (!root.json) {
            root.text = raw;
            root.level = "normal";
            root.meter = -1;
            root.failure = "";
            root.hasResult = true;
            return;
        }
        let value = null;
        try {
            value = JSON.parse(raw);
        } catch (e) {
            root._fail("output is not JSON: " + raw.slice(0, 80));
            return;
        }
        if (value === null || typeof value !== "object" || typeof value.text !== "string") {
            root._fail("JSON output has no string `text`");
            return;
        }
        root.text = value.text;
        root.level = value.state === "warning" || value.state === "danger" ? value.state : "normal";
        root.meter = typeof value.meter === "number" ? Math.max(0, Math.min(1, value.meter)) : -1;
        root.failure = "";
        root.hasResult = true;
    }

    function _fail(why: string): void {
        if (root.failure !== why)
            console.warn("vogix: custom/" + root.name + ": " + why);
        root.failure = why;
        root.hasResult = true;
    }

    function _read(line: string): void {
        if (root.stream) {
            root._publish(line.trim());
            return;
        }
        if (root._firstLine === "" && line.trim() !== "")
            root._firstLine = line.trim();
        if (root.json && root._all.length < root._jsonCap)
            root._all += line + "\n";
    }

    // exitStatus: 0 = normal exit, 1 = killed by a signal (QProcess).
    function _exited(exitCode: int, exitStatus: int): void {
        if (root._stopping) {
            root._stopping = false;
        } else if (exitStatus !== 0) {
            root._fail("killed");
        } else if (exitCode !== 0) {
            root._fail("exited " + exitCode);
        } else if (!root.stream) {
            root._publish(root.json ? root._all.trim() : root._firstLine);
        }
        if (root._queued) {
            root._queued = false;
            root.trigger();
        }
    }

    // Connected here rather than as an `onExited` handler: quickshell's
    // qmltypes do not carry QProcess::ExitStatus, so a declarative handler
    // for that signal cannot be compiled.
    Component.onCompleted: {
        proc.exited.connect(root._exited);
        if (root.active)
            root.trigger();
    }

    onActiveChanged: {
        if (active)
            trigger();
        else
            _stop(false);
    }

    onCommandChanged: {
        if (active)
            _stop(true);
    }

    Process {
        id: proc
        command: ["sh", "-c", root.command]
        stdout: SplitParser {
            onRead: line => root._read(line)
        }
    }

    Process {
        id: clickProc
        command: ["sh", "-c", root.clickCommand]
        onRunningChanged: {
            if (!running)
                root.trigger();
        }
    }

    Timer {
        interval: Math.max(1, root.intervalSec) * 1000
        running: root.active && root.intervalSec > 0
        repeat: true
        onTriggered: root.trigger()
    }

    // Change notification only: nothing is read from these files.
    Variants {
        model: root.active ? root.watch : []

        FileView {
            id: watched

            required property var modelData

            path: watched.modelData
            preload: false
            printErrors: false
            watchChanges: true
            onFileChanged: root.trigger()
        }
    }
}

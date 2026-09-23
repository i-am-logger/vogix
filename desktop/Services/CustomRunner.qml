pragma ComponentBehavior: Bound
// One custom cell's command (desktop.json `custom.<name>`), shared by every
// cell that shows it. The command runs only while one of those cells is on
// a live bar (`active`: shown, not parked, the screen in use); nothing runs
// while every bar showing it is parked, locked away or switched off.
//
// A run starts when the cell becomes live and its last result is older
// than its `interval` (or it has none), once the result reaches that age
// while live, when a `watch` file changes, after the cell's click action
// finishes, and on `vogix desktop custom refresh`. A watch change or a
// refresh that arrives while the cell is not live, and a run cut short by
// its bar leaving the screen, leave the result stale: the command runs as
// soon as the cell is live again. A trigger during a one-shot run queues
// exactly one re-run, so a burst of file changes costs one more run, never
// a pile-up. A `stream` command stays up while live and publishes every
// line it prints; it starts again whenever the cell becomes live, and a
// trigger relaunches it only once it has exited.
//
// A command runs as the leader of a process group of its own, and ending
// a run (the cell leaving the screen, a changed command, the cell removed
// from desktop.json) signals that group: every stage of a pipeline or
// compound command ends with it, not only `sh`.
import QtQuick
import Quickshell
import Quickshell.Io

Scope {
    id: root

    required property string name
    property var def: ({})
    // Some cell shows this command (on any bar, live or not).
    property bool placed: false
    // Some cell showing it is on a live bar.
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
    readonly property string status: !placed && !hasResult ? "inactive"
        : failure !== "" ? "failed: " + failure
        : hasResult ? text
        : "pending"

    // When the latest result (a value or a failure) landed, epoch ms.
    property real _resultAt: 0
    // Something the result depends on changed while nothing could run.
    property bool _stale: false
    property bool _queued: false
    property bool _stopping: false
    // A one-shot run's output, published once the run exits: its first
    // non-empty line (text), or the whole of it up to _jsonCap (json).
    property string _firstLine: ""
    property string _all: ""
    readonly property int _jsonCap: 64 * 1024

    // Run the command now, or, while no bar showing it is live, as soon as
    // one is.
    function trigger(): void {
        if (root.command === "")
            return;
        if (!root.active) {
            root._stale = true;
            return;
        }
        if (proc.running) {
            if (!root.stream)
                root._queued = true;
            return;
        }
        root._stale = false;
        root._firstLine = "";
        root._all = "";
        due.stop();
        proc.running = true;
    }

    // The result is at least `interval` old.
    function _due(): bool {
        return root.intervalSec > 0 && Date.now() - root._resultAt >= root.intervalSec * 1000;
    }

    // Wake when the result reaches its interval's age.
    function _schedule(): void {
        if (!root.active || root.intervalSec <= 0 || proc.running) {
            due.stop();
            return;
        }
        due.interval = Math.max(1, root.intervalSec * 1000 - (Date.now() - root._resultAt));
        due.restart();
    }

    // SIGTERM to the running command's process group. quickshell signals
    // only `sh` (SIGTERM on a stop, SIGKILL on destruction); the stages it
    // started would outlive it, holding the command's pipe. The shell's
    // own `kill` takes a group the same way everywhere; procps' `kill`
    // refuses `--`.
    function _endGroup(): void {
        const pid = proc.processId;
        if (typeof pid === "number" && pid > 0)
            Quickshell.execDetached(["sh", "-c", "kill -TERM -- \"-$1\"", "sh", String(pid)]);
    }

    // A changed command or a cell leaving the screen ends the current run
    // without counting it as a failure; a changed command starts the new
    // one.
    function _stop(thenRun: bool): void {
        root._queued = thenRun;
        if (proc.running) {
            root._stopping = true;
            root._endGroup();
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

    function _landed(): void {
        root.hasResult = true;
        root._resultAt = Date.now();
    }

    function _publish(raw: string): void {
        if (!root.json) {
            root.text = raw;
            root.level = "normal";
            root.meter = -1;
            root.failure = "";
            root._landed();
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
        root._landed();
    }

    function _fail(why: string): void {
        if (root.failure !== why)
            console.warn("vogix: custom/" + root.name + ": " + why);
        root.failure = why;
        root._landed();
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
        // A queued trigger, or a run owed since the cell left the screen and
        // came back before this one finished stopping.
        if (root._queued || (root.active && root._stale)) {
            root._queued = false;
            root.trigger();
        } else {
            root._schedule();
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

    // The cell was removed from desktop.json while its command ran.
    Component.onDestruction: {
        if (proc.running)
            root._endGroup();
    }

    onActiveChanged: {
        if (active) {
            if (root.stream || root._stale || !root.hasResult || root._due())
                root.trigger();
            else
                root._schedule();
        } else {
            due.stop();
            // A run cut short owes its result.
            if (proc.running)
                root._stale = true;
            root._stop(false);
        }
    }

    onCommandChanged: {
        if (active)
            _stop(true);
    }

    // setsid: sh leads a new process group, which every process it starts
    // joins (see _endGroup).
    Process {
        id: proc
        command: ["setsid", "sh", "-c", root.command]
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
        id: due
        onTriggered: root.trigger()
    }

    // Change notification only: nothing is read from these files. Watched
    // while any cell shows the command, so a change while none is live is
    // not missed.
    Variants {
        model: root.placed ? root.watch : []

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

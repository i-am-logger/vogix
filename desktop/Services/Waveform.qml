pragma Singleton
// The oscilloscope's sample tap: pw-record on the default sink MONITOR
// (stream.capture.sink), 8 kHz mono s16 turned into text by od, one line
// per 256 samples (32 ms), so the scope updates at ~31 fps as the data
// arrives with no timer of its own. The stream is tagged
// node.name=vogix-scope so the privacy indicator can tell it from a real
// recording. Ref-counted — capture runs only while a scope widget is on
// screen and something is playing. A line identical to the one before
// (digital silence, a held tone) is not parsed again. AudioTap supervises
// the process: it waits for PipeWire and a default sink, and comes back
// after an exit nobody asked for.
import QtQuick
import Quickshell

Singleton {
    id: root

    property int refs: 0

    function acquire(): void {
        refs++;
    }

    function release(): void {
        refs = Math.max(0, refs - 1);
    }

    readonly property bool active: refs > 0

    // off: no visible widget · idle: visible, nothing playing · otherwise
    // the tap's own state.
    function status(): string {
        if (!root.active)
            return "off";
        if (!Audio.playing)
            return "idle";
        return tap.status();
    }

    // -1..1 samples, the two most recent lines (512 samples = 64 ms).
    property list<real> waveform: []
    // The last line's samples and text.
    property var _prev: []
    property string _lastLine: ""
    // The window already shows the repeated line twice.
    property bool _steady: false

    function _clear(): void {
        root._prev = [];
        root._lastLine = "";
        root._steady = false;
        root.waveform = [];
    }

    onActiveChanged: {
        if (!active)
            _clear();
    }

    AudioTap {
        id: tap

        name: "pw-record"
        wanted: root.active && Audio.playing
        // bash execs into pw-record, so the process quickshell holds is the
        // capture itself; od runs in a process substitution and ends on
        // pw-record's EOF. Both run unbuffered/line-buffered (stdbuf), or
        // stdio would hold 4 KiB — a quarter second — before od saw it.
        command: ["bash", "-c",
            "exec stdbuf -o0 pw-record --raw -P '{ stream.capture.sink=true node.name=vogix-scope }' --format=s16 --rate=8000 --channels=1 - > >(exec stdbuf -oL od -An -td2 -v -w512)"]

        onRunningChanged: {
            if (!running)
                root._clear();
        }

        onLine: data => {
            if (data === root._lastLine) {
                if (!root._steady) {
                    root._steady = true;
                    root.waveform = root._prev.concat(root._prev);
                }
                return;
            }
            root._steady = false;
            root._lastLine = data;
            const cur = [];
            for (const p of data.trim().split(/\s+/)) {
                const n = Number(p);
                if (!Number.isNaN(n))
                    cur.push(n / 32768);
            }
            if (root._prev.length > 0)
                root.waveform = root._prev.concat(cur);
            root._prev = cur;
        }
    }
}

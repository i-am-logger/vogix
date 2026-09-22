pragma Singleton
// The oscilloscope's sample tap: pw-record on the default sink MONITOR
// (stream.capture.sink), 8 kHz mono s16 piped through od into text the
// shell can parse. The stream is tagged node.name=vogix-scope so the
// privacy indicator can tell it from a real recording. Ref-counted —
// capture runs only while a scope widget is on screen; UI writes are
// throttled to ~30 fps regardless of sample rate. AudioTap supervises
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

    // -1..1 samples, most recent window (512 samples = 64 ms at 8 kHz).
    property list<real> waveform: []
    property var _buf: []

    function _clear(): void {
        root._buf = [];
        root.waveform = [];
    }

    onActiveChanged: {
        if (!active)
            _clear();
    }

    AudioTap {
        id: tap

        name: "pw-record"
        wanted: root.active
        // bash execs into pw-record, so the process quickshell holds is the
        // capture itself; od runs in a process substitution and ends on
        // pw-record's EOF.
        command: ["bash", "-c",
            "exec pw-record -P '{ stream.capture.sink=true node.name=vogix-scope }' --format=s16 --rate=8000 --channels=1 - > >(exec od -An -td2 -v -w64)"]

        onRunningChanged: {
            if (!running)
                root._clear();
        }

        onLine: data => {
            const parts = data.trim().split(/\s+/);
            for (const p of parts) {
                const n = Number(p);
                if (!Number.isNaN(n))
                    root._buf.push(n / 32768);
            }
            if (root._buf.length > 2048)
                root._buf = root._buf.slice(-1024);
        }
    }

    function status(): string {
        return tap.status();
    }

    Timer {
        interval: 33
        running: root.active
        repeat: true
        onTriggered: {
            if (root._buf.length >= 512)
                root.waveform = root._buf.slice(-512);
        }
    }
}

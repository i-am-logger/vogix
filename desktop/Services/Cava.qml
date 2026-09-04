pragma Singleton
// The audio spectrum: a cava subprocess in raw-ascii mode, ref-counted so
// it runs ONLY while a spectrum widget is on screen, killed (and the bars
// zero-filled) the moment the last one goes. Config is inlined over
// stdin — no file to manage. Frame writes skip when nothing changed, so
// silence costs no repaints.
import QtQuick
import Quickshell
import Quickshell.Io
import qs.Vogix

Singleton {
    id: root

    property int refs: 0

    function acquire(): void {
        refs++;
    }

    function release(): void {
        refs = Math.max(0, refs - 1);
    }

    readonly property var spectrumConf: (Config.doc.meters ?? {}).spectrum ?? ({})
    // Bars PER CHANNEL — cava runs stereo, emitting the left channel
    // reversed then the right (its mirrored display convention).
    readonly property int bars: spectrumConf.bars ?? 16
    readonly property bool active: (spectrumConf.enable ?? true) && refs > 0

    // 0..1 per bar, length == bars*2 (the raw mirrored stereo array);
    // zero-filled while inactive.
    property list<real> values: []

    // Per-channel views, low→high frequency order.
    readonly property var leftValues: values.slice(0, values.length >> 1).reverse()
    readonly property var rightValues: values.slice(values.length >> 1)

    // VU BALLISTICS, taken verbatim from Peaks: instant attack, bar release
    // 3.0 FS/s, cap holds 0.53 s then falls 0.75 FS/s (the 4:1 bar/cap ratio).
    // The spectrum falls at the VU rail's rate because the two read as one
    // instrument family — a second set of constants here would make the bars
    // and the meters disagree on screen for the same sound.
    readonly property real barReleasePerSec: 3.0
    readonly property real capFallPerSec: 0.75
    readonly property real capHoldSec: 0.53

    // `values` is the instantaneous frame; these carry the ballistics.
    property list<real> heldValues: []
    property list<real> capValues: []
    property var _capHoldLeft: []

    // Driven by a TIMER, not by the frame callback. Frames are deduped
    // against `_last`, so a steady (or silent) stream stops emitting — and a
    // decay driven off arrivals would freeze at the last value instead of
    // falling to zero, which is the one thing a release curve must not do.
    Timer {
        interval: 40   // cava's own 25 fps
        running: root.active
        repeat: true
        onTriggered: root._advance(interval / 1000)
    }

    function _advance(dt: real): void {
        const src = root.values;
        const n = src.length;
        if (n === 0)
            return;
        const held = root.heldValues.length === n ? root.heldValues.slice() : Array(n).fill(0);
        const caps = root.capValues.length === n ? root.capValues.slice() : Array(n).fill(0);
        const hold = root._capHoldLeft.length === n ? root._capHoldLeft.slice() : Array(n).fill(0);
        let moved = false;
        for (let i = 0; i < n; i++) {
            const v = src[i];
            // Instant attack, timed release.
            const h = v >= held[i] ? v : Math.max(v, held[i] - root.barReleasePerSec * dt);
            if (Math.round(h * 40) !== Math.round(held[i] * 40))
                moved = true;
            held[i] = h;
            if (h >= caps[i]) {
                caps[i] = h;
                hold[i] = root.capHoldSec;
            } else if (hold[i] > 0) {
                hold[i] -= dt;
            } else {
                const c = Math.max(h, caps[i] - root.capFallPerSec * dt);
                if (Math.round(c * 40) !== Math.round(caps[i] * 40))
                    moved = true;
                caps[i] = c;
            }
        }
        root._capHoldLeft = hold;
        // Quantized to 1/40 like Peaks: once everything has settled the writes
        // stop, so silence still costs no repaints.
        if (moved) {
            root.heldValues = held;
            root.capValues = caps;
        }
    }

    property string _last: ""

    onActiveChanged: {
        if (!active) {
            values = Array(bars * 2).fill(0);
            heldValues = [];
            capValues = [];
            _capHoldLeft = [];
            _last = "";
        }
    }

    Process {
        id: proc

        running: root.active
        command: ["sh", "-c",
            "printf '[general]\\nframerate=25\\nbars=%d\\n[output]\\nmethod=raw\\ndata_format=ascii\\nascii_max_range=100\\nchannels=stereo\\n[smoothing]\\nnoise_reduction=35\\nmonstercat=1.5\\n' "
            + (root.bars * 2) + " | cava -p /dev/stdin"]

        stdout: SplitParser {
            onRead: line => {
                if (line === root._last)
                    return;
                root._last = line;
                root.values = line.split(";").filter(s => s !== "")
                    .map(s => Math.max(0, Math.min(1, Number(s) / 100)));
            }
        }

        // running flips false when the subprocess dies on its own too — an
        // EOF'd cava while a spectrum widget still wants it is worth a line.
        onRunningChanged: {
            if (!running && root.active)
                console.warn("vogix: cava exited while a spectrum widget is visible");
        }
    }
}

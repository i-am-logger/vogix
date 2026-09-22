pragma Singleton
// The audio spectrum: a cava subprocess in raw-ascii mode, ref-counted so
// it runs ONLY while a spectrum widget is on screen, killed (and the bars
// zero-filled) the moment the last one goes. Config is inlined over
// stdin — no file to manage. Frame writes skip when nothing changed, so
// silence costs no repaints.
import QtQuick
import Quickshell
import Quickshell.Io
import qs.Components
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
    // zero-filled while inactive. SpectrumWidget derives its per-channel
    // views from this.
    property list<real> values: []

    // The ballistic view of `values`, what the widgets draw: the curve the
    // VU meters fall on (qs.Components Ballistics), because the spectrum
    // and the rail read as one instrument family.
    property list<real> heldValues: []
    property list<real> capValues: []

    // Ballistics state, advanced in place every tick; the published lists
    // above are written only when a bar or cap moved by a visible step.
    property var _level: []
    property var _cap: []
    property var _hold: []

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
        if (root._level.length !== n) {
            root._level = Array(n).fill(0);
            root._cap = Array(n).fill(0);
            root._hold = Array(n).fill(0);
        }
        if (Ballistics.advance(src, root._level, root._cap, root._hold, dt)) {
            root.heldValues = root._level;
            root.capValues = root._cap;
        }
    }

    property string _last: ""

    onActiveChanged: {
        if (!active) {
            values = Array(bars * 2).fill(0);
            heldValues = [];
            capValues = [];
            _level = [];
            _cap = [];
            _hold = [];
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

pragma Singleton
// The audio spectrum: cava in raw-ascii mode on the default sink's
// monitor, ref-counted so it runs ONLY while a spectrum widget is on
// screen, stopped (and the bars zero-filled) the moment the last one goes.
// The process is supervised by AudioTap: it waits for PipeWire and a
// default sink, and comes back after an exit nobody asked for. Config is
// inlined over stdin — no file to manage. Frame writes skip when nothing
// changed, so silence costs no repaints.
import QtQuick
import Quickshell
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

    // Inline cava config, fed to `-p /dev/stdin` through a heredoc so the
    // shell can exec into cava: the tap is then cava itself.
    readonly property string _config: [
        "[general]", "framerate=25", "bars=" + (root.bars * 2),
        "[input]", "method=pipewire", "source=auto",
        "[output]", "method=raw", "data_format=ascii", "ascii_max_range=100", "channels=stereo",
        "[smoothing]", "noise_reduction=35", "monstercat=1.5", ""
    ].join("\n")

    AudioTap {
        id: tap

        name: "cava"
        wanted: root.active
        command: ["sh", "-c", "exec cava -p /dev/stdin <<'EOF'\n" + root._config + "EOF\n"]

        // A dead tap must not leave its last frame standing: zero it and
        // let the ballistics fall.
        onRunningChanged: {
            if (!running) {
                root.values = Array(root.bars * 2).fill(0);
                root._last = "";
            }
        }

        onLine: data => {
            if (data === root._last)
                return;
            root._last = data;
            root.values = data.split(";").filter(s => s !== "")
                .map(s => Math.max(0, Math.min(1, Number(s) / 100)));
        }
    }

    function status(): string {
        return tap.status();
    }
}

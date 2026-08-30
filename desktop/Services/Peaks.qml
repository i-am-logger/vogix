pragma Singleton
// Live VU state for the default sink and source, from quickshell's native
// PwNodePeakMonitor — no external tap. Monitor peaks arrive
// cbrt-compressed (peak.cpp: visualPeak = cbrt(peak)), so true
// dB = 20·log10(raw³) = 60·log10(raw). Levels are positions in the
// [floorDb, 0] window with cava-peaks ballistics: instant attack, bar
// release 3.0 FS/s, cap holds 0.53 s then falls 0.75 FS/s (the 4:1
// bar/cap ratio). Values quantize to 1/40 and skip unchanged writes, so
// a silent stream costs zero repaints; monitors are ref-counted and
// capture only while a VU widget is on screen.
import QtQuick
import Quickshell
import Quickshell.Services.Pipewire
import qs.Vogix

Singleton {
    id: root

    property int outRefs: 0
    property int micRefs: 0

    function acquire(kind: string): void {
        if (kind === "mic")
            micRefs++;
        else
            outRefs++;
    }

    function release(kind: string): void {
        if (kind === "mic")
            micRefs = Math.max(0, micRefs - 1);
        else
            outRefs = Math.max(0, outRefs - 1);
    }

    readonly property real floorDb: ((Config.doc.meters ?? {}).vu ?? {}).floorDb ?? -40

    // Public state, 0..1 in the dB window, quantized 1/40. The output is
    // STEREO (L/R per channel); outLevel/outCap are the max of the two
    // for mono consumers. Mic is mono.
    property real outL: 0
    property real outR: 0
    property real outCapL: 0
    property real outCapR: 0
    property real outLevel: 0
    property real outCap: 0
    property real micLevel: 0
    property real micCap: 0

    // The dB a window position displays.
    function dbOf(level: real): real {
        return root.floorDb * (1 - level);
    }

    // Raw (unquantized) ballistics state, per channel.
    property real _outL: 0
    property real _outR: 0
    property real _outCapL: 0
    property real _outCapR: 0
    property real _outHoldL: 0
    property real _outHoldR: 0
    property real _micLevel: 0
    property real _micCap: 0
    property real _micHold: 0

    function _norm(raw: real): real {
        if (raw <= 0)
            return 0;
        const db = 60 * Math.log10(raw);
        return Math.max(0, Math.min(1, 1 - db / root.floorDb));
    }

    PwNodePeakMonitor {
        id: outMon
        node: Pipewire.defaultAudioSink
        enabled: root.outRefs > 0
        // Instant attack per channel; the tick only handles decay. A mono
        // stream feeds both columns.
        onPeaksChanged: {
            const l = peaks[0] ?? 0;
            const r = peaks[1] ?? l;
            root._outL = Math.max(root._outL, root._norm(l));
            root._outR = Math.max(root._outR, root._norm(r));
        }
    }

    PwNodePeakMonitor {
        id: micMon
        node: Pipewire.defaultAudioSource
        enabled: root.micRefs > 0
        onPeakChanged: root._micLevel = Math.max(root._micLevel, root._norm(peak))
    }

    Timer {
        // The 30 fps ballistics tick (NOT a FrameAnimation: that fires at
        // the display's full refresh rate — 160 Hz here — and the decay
        // math neither needs nor deserves that). Runs only while
        // something is audible or still decaying.
        interval: 33
        repeat: true
        running: (root.outRefs > 0 || root.micRefs > 0)
            && (root._outL > 0 || root._outR > 0 || root._outCapL > 0 || root._outCapR > 0
                || root._micLevel > 0 || root._micCap > 0
                || outMon.peak > 0 || micMon.peak > 0)

        onTriggered: {
            const dt = 0.033;

            const tl = root._norm(outMon.peaks[0] ?? 0);
            const tr = root._norm(outMon.peaks[1] ?? (outMon.peaks[0] ?? 0));

            root._outL = Math.max(tl, root._outL - 3.0 * dt);
            if (root._outL >= root._outCapL) {
                root._outCapL = root._outL;
                root._outHoldL = 0.53;
            } else if ((root._outHoldL -= dt) <= 0) {
                root._outCapL = Math.max(root._outL, root._outCapL - 0.75 * dt);
            }

            root._outR = Math.max(tr, root._outR - 3.0 * dt);
            if (root._outR >= root._outCapR) {
                root._outCapR = root._outR;
                root._outHoldR = 0.53;
            } else if ((root._outHoldR -= dt) <= 0) {
                root._outCapR = Math.max(root._outR, root._outCapR - 0.75 * dt);
            }

            root._micLevel = Math.max(root._norm(micMon.peak), root._micLevel - 3.0 * dt);
            if (root._micLevel >= root._micCap) {
                root._micCap = root._micLevel;
                root._micHold = 0.53;
            } else if ((root._micHold -= dt) <= 0) {
                root._micCap = Math.max(root._micLevel, root._micCap - 0.75 * dt);
            }

            const q = v => Math.round(v * 40) / 40;
            const write = (name, v) => {
                if (v !== root[name])
                    root[name] = v;
            };
            write("outL", q(root._outL));
            write("outR", q(root._outR));
            write("outCapL", q(root._outCapL));
            write("outCapR", q(root._outCapR));
            write("outLevel", Math.max(root.outL, root.outR));
            write("outCap", Math.max(root.outCapL, root.outCapR));
            write("micLevel", q(root._micLevel));
            write("micCap", q(root._micCap));
        }

        onRunningChanged: {
            if (!running) {
                // Settle to zero so nothing freezes mid-decay.
                root._outL = 0; root._outR = 0; root._outCapL = 0; root._outCapR = 0;
                root._micLevel = 0; root._micCap = 0;
                root.outL = 0; root.outR = 0; root.outCapL = 0; root.outCapR = 0;
                root.outLevel = 0; root.outCap = 0;
                root.micLevel = 0; root.micCap = 0;
            }
        }
    }
}

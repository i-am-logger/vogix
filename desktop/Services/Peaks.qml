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

    // Public state, 0..1 in the dB window, quantized 1/40.
    property real outLevel: 0
    property real outCap: 0
    property real micLevel: 0
    property real micCap: 0

    // The dB a window position displays.
    function dbOf(level: real): real {
        return root.floorDb * (1 - level);
    }

    // Raw (unquantized) ballistics state.
    property real _outLevel: 0
    property real _outCap: 0
    property real _outHold: 0
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
        // Instant attack; the frame loop only handles decay.
        onPeakChanged: root._outLevel = Math.max(root._outLevel, root._norm(peak))
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
            && (root._outLevel > 0 || root._outCap > 0 || root._micLevel > 0 || root._micCap > 0
                || outMon.peak > 0 || micMon.peak > 0)

        onTriggered: {
            const dt = 0.033;

            root._outLevel = Math.max(root._norm(outMon.peak), root._outLevel - 3.0 * dt);
            if (root._outLevel >= root._outCap) {
                root._outCap = root._outLevel;
                root._outHold = 0.53;
            } else if ((root._outHold -= dt) <= 0) {
                root._outCap = Math.max(root._outLevel, root._outCap - 0.75 * dt);
            }

            root._micLevel = Math.max(root._norm(micMon.peak), root._micLevel - 3.0 * dt);
            if (root._micLevel >= root._micCap) {
                root._micCap = root._micLevel;
                root._micHold = 0.53;
            } else if ((root._micHold -= dt) <= 0) {
                root._micCap = Math.max(root._micLevel, root._micCap - 0.75 * dt);
            }

            const q = v => Math.round(v * 40) / 40;
            const ol = q(root._outLevel);
            if (ol !== root.outLevel)
                root.outLevel = ol;
            const oc = q(root._outCap);
            if (oc !== root.outCap)
                root.outCap = oc;
            const ml = q(root._micLevel);
            if (ml !== root.micLevel)
                root.micLevel = ml;
            const mc = q(root._micCap);
            if (mc !== root.micCap)
                root.micCap = mc;
        }

        onRunningChanged: {
            if (!running) {
                // Settle to zero so nothing freezes mid-decay.
                root._outLevel = 0; root._outCap = 0;
                root._micLevel = 0; root._micCap = 0;
                root.outLevel = 0; root.outCap = 0;
                root.micLevel = 0; root.micCap = 0;
            }
        }
    }
}

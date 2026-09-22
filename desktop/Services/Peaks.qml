pragma Singleton
// Live VU state for the default sink and source, from quickshell's native
// PwNodePeakMonitor — no external tap. Monitor peaks arrive
// cbrt-compressed (peak.cpp: visualPeak = cbrt(peak)), so true
// dB = 20·log10(raw³) = 60·log10(raw). Levels are positions in the
// [floorDb, 0] window, moved by the meter ballistics the spectrum
// shares (qs.Components Ballistics); published values quantize and skip
// unchanged writes. Monitors are ref-counted and capture only while a VU
// widget is on screen — the output one only while something plays — and
// the ballistics tick runs only while a level is moving, so a silent or
// steady meter costs no wakeups of its own.
import QtQuick
import Quickshell
import Quickshell.Services.Pipewire
import qs.Components
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

    // With no playback stream the sink's monitor carries only silence.
    readonly property bool outActive: root.outRefs > 0 && Audio.playing
    readonly property bool micActive: root.micRefs > 0

    // off: no visible widget · idle: visible, nothing playing · on.
    function outStatus(): string {
        return root.outRefs === 0 ? "off" : (Audio.playing ? "on" : "idle");
    }

    function micStatus(): string {
        return root.micActive ? "on" : "off";
    }

    readonly property real floorDb: ((Config.doc.meters ?? {}).vu ?? {}).floorDb ?? -40

    // Public state, 0..1 in the dB window, quantized. The output is
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

    // Ballistics state, one entry per channel in one order throughout:
    // out L, out R, mic. `_pending` is the highest level each channel
    // reached since the last tick, so a transient between ticks lands.
    property var _pending: [0, 0, 0]
    property var _level: [0, 0, 0]
    property var _cap: [0, 0, 0]
    property var _hold: [0, 0, 0]
    // Nothing will move until a peak differs from the settled state.
    property bool _settled: true

    function _norm(raw: real): real {
        if (raw <= 0)
            return 0;
        const db = 60 * Math.log10(raw);
        return Math.max(0, Math.min(1, 1 - db / root.floorDb));
    }

    // One channel's new position in the window. Peaks below the floor
    // normalize to 0, so sub-floor noise on a settled meter changes
    // nothing and wakes nothing.
    function _heard(channel: int, level: real): void {
        if (level > root._pending[channel])
            root._pending[channel] = level;
        if (level !== root._level[channel] || root._cap[channel] !== root._level[channel])
            root._settled = false;
    }

    function _publish(): void {
        const write = (name, v) => {
            if (v !== root[name])
                root[name] = v;
        };
        write("outL", Ballistics.quantize(root._level[0]));
        write("outR", Ballistics.quantize(root._level[1]));
        write("outCapL", Ballistics.quantize(root._cap[0]));
        write("outCapR", Ballistics.quantize(root._cap[1]));
        write("outLevel", Math.max(root.outL, root.outR));
        write("outCap", Math.max(root.outCapL, root.outCapR));
        write("micLevel", Ballistics.quantize(root._level[2]));
        write("micCap", Ballistics.quantize(root._cap[2]));
    }

    // A monitor that stops leaves its channels at zero, not mid-decay.
    function _zero(channels: var): void {
        for (const i of channels) {
            root._pending[i] = 0;
            root._level[i] = 0;
            root._cap[i] = 0;
            root._hold[i] = 0;
        }
        root._publish();
    }

    onOutActiveChanged: {
        if (!outActive)
            _zero([0, 1]);
    }

    onMicActiveChanged: {
        if (!micActive)
            _zero([2]);
    }

    PwNodePeakMonitor {
        id: outMon
        node: Pipewire.defaultAudioSink
        enabled: root.outActive
        // A mono stream feeds both columns.
        onPeaksChanged: {
            const l = root._norm(peaks[0] ?? 0);
            root._heard(0, l);
            root._heard(1, peaks.length > 1 ? root._norm(peaks[1]) : l);
        }
    }

    PwNodePeakMonitor {
        id: micMon
        node: Pipewire.defaultAudioSource
        enabled: root.micActive
        onPeakChanged: root._heard(2, root._norm(peak))
    }

    Timer {
        // The 30 fps ballistics tick (NOT a FrameAnimation: that fires at
        // the display's full refresh rate — 160 Hz here — and the decay
        // math neither needs nor deserves that).
        interval: 33
        repeat: true
        running: (root.outActive || root.micActive) && !root._settled

        onTriggered: {
            const now = [
                root._norm(outMon.peaks[0] ?? 0),
                root._norm(outMon.peaks[1] ?? (outMon.peaks[0] ?? 0)),
                root._norm(micMon.peak)
            ];
            const target = now.map((v, i) => Math.max(v, root._pending[i]));
            root._pending = [0, 0, 0];
            Ballistics.advance(target, root._level, root._cap, root._hold, interval / 1000);
            root._publish();
            root._settled = Ballistics.settled(now, root._level, root._cap);
        }
    }
}

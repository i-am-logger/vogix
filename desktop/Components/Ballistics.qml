pragma Singleton
// Meter ballistics: the one definition the VU meters (Peaks) and the
// spectrum (Cava) share, so the bars and the meters beside them fall
// together for the same sound. Instant attack; the bar releases at
// 3.0 full-scale/s; the peak cap holds 0.53 s, then falls at
// 0.75 full-scale/s (the fleet's cava-peaks tuning, a 4:1 bar/cap
// ratio). Levels are fractions of full scale. Published values quantize
// to 1/40, so a settled meter stops writing. The behaviour half of the
// meter primitives beside it, and a shell design constant like Metrics:
// the v2 Rust shell reimplements the same table.
//
// Plain QtQuick in a module with no quickshell singletons, so
// qmltestrunner pins it (tests/desktop/tst_ballistics.qml) without a
// quickshell engine.
import QtQuick

QtObject {
    id: root

    readonly property real barReleasePerSec: 3.0
    readonly property real capHoldSec: 0.53
    readonly property real capFallPerSec: 0.75
    // Published resolution: 1/steps of full scale.
    readonly property int steps: 40

    function quantize(v: real): real {
        return Math.round(v * root.steps) / root.steps;
    }

    // Advances every channel by `dt` seconds, IN PLACE. `target` holds each
    // channel's instantaneous level; `level`, `cap` and `hold` (seconds of
    // cap hold left) carry the state between calls, one entry per channel.
    // Returns whether any channel's quantized level or cap moved.
    function advance(target: var, level: var, cap: var, hold: var, dt: real): bool {
        let moved = false;
        for (let i = 0; i < target.length; i++) {
            const t = target[i];
            const was = level[i];
            const l = t >= was ? t : Math.max(t, was - root.barReleasePerSec * dt);
            if (root.quantize(l) !== root.quantize(was))
                moved = true;
            level[i] = l;
            if (l >= cap[i]) {
                if (root.quantize(l) !== root.quantize(cap[i]))
                    moved = true;
                cap[i] = l;
                hold[i] = root.capHoldSec;
            } else if (hold[i] > 0) {
                hold[i] -= dt;
            } else {
                const c = Math.max(l, cap[i] - root.capFallPerSec * dt);
                if (root.quantize(c) !== root.quantize(cap[i]))
                    moved = true;
                cap[i] = c;
            }
        }
        return moved;
    }
}

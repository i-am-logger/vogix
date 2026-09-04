// The HUD's core meter: discrete lit segments with a peak-hold cap.
// Geometry from the fleet's segment spec (6px segments, 3px gaps, scaled by
// the type root); coloring is POSITIONAL — segments below warnAt light in
// `low`, then `mid`, then `high` — with unlit troughs and a cap segment.
// Repaints ONLY when the lit count or cap index changes.
import QtQuick
import qs.Vogix

Canvas {
    id: root

    property real value: 0        // 0..1
    property real peak: -1        // 0..1 cap position, -1 = no cap
    property bool vertical: false
    property real warnAt: 0.7
    property real dangerAt: 0.85

    property color low: "#ff00ff"
    property color mid: "#ff00ff"
    property color high: "#ff00ff"
    property color unlit: "#ff00ff"
    property color capColor: "#ff00ff"

    // Segment geometry, scaled off the type root (base 16 → 6px/3px).
    readonly property int segLength: Math.max(4, Math.round(Metrics.body * 0.375))
    readonly property int segGap: Math.max(2, Math.round(Metrics.body * 0.1875))
    readonly property int pitch: segLength + segGap
    readonly property int segCount: Math.max(1, Math.floor(((vertical ? height : width) + segGap) / pitch))

    // Quantized state — the ONLY repaint triggers.
    readonly property int litCount: Math.round(Math.min(1, Math.max(0, value)) * segCount)
    readonly property int capIndex: peak < 0 ? -1 : Math.min(segCount - 1, Math.round(peak * segCount))

    onLitCountChanged: requestPaint()
    onCapIndexChanged: requestPaint()
    onWidthChanged: requestPaint()
    onHeightChanged: requestPaint()
    Component.onCompleted: requestPaint()

    onPaint: {
        const ctx = getContext("2d");
        ctx.clearRect(0, 0, width, height);
        const thick = vertical ? width : height;
        for (let i = 0; i < segCount; i++) {
            const frac = (i + 1) / segCount;
            let c = root.unlit;
            if (i < litCount)
                c = frac > dangerAt ? root.high : (frac > warnAt ? root.mid : root.low);
            if (i === capIndex)
                c = root.capColor;
            ctx.fillStyle = c;
            if (vertical)
                ctx.fillRect(0, height - (i * pitch) - segLength, thick, segLength);
            else
                ctx.fillRect(i * pitch, 0, segLength, thick);
        }
    }
}

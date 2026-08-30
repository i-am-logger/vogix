// A small history graph: primary series as a stroked line over a soft
// area fill, optional dashed secondary series (the NET up/down pair).
// Values are 0..1; the caller reassigns whole arrays (ring-buffer style),
// which is the repaint trigger.
import QtQuick

Canvas {
    id: root

    property var values: []       // list<real> 0..1, oldest first
    property var secondary: []    // optional second series, dashed
    property color lineColor: "#ff00ff"
    property color secondaryColor: "#ff00ff"
    property real fillAlpha: 0.14

    onValuesChanged: requestPaint()
    onSecondaryChanged: requestPaint()
    onWidthChanged: requestPaint()
    onHeightChanged: requestPaint()

    function plot(ctx, series, color, dashed, fill) {
        const n = series.length;
        if (n < 2)
            return;
        const dx = width / (n - 1);
        // Plot into 80% of the height so peaks never kiss the frame.
        const y = v => height - Math.min(1, Math.max(0, v)) * height * 0.8 - height * 0.1;
        ctx.beginPath();
        ctx.moveTo(0, y(series[0]));
        for (let i = 1; i < n; i++)
            ctx.lineTo(i * dx, y(series[i]));
        ctx.strokeStyle = color;
        ctx.lineWidth = 1;
        ctx.setLineDash(dashed ? [4, 4] : []);
        ctx.stroke();
        ctx.setLineDash([]);
        if (fill) {
            ctx.lineTo(width, height);
            ctx.lineTo(0, height);
            ctx.closePath();
            ctx.fillStyle = Qt.alpha(color, root.fillAlpha);
            ctx.fill();
        }
    }

    onPaint: {
        const ctx = getContext("2d");
        ctx.clearRect(0, 0, width, height);
        plot(ctx, root.values, root.lineColor, false, true);
        if (root.secondary.length > 1)
            plot(ctx, root.secondary, root.secondaryColor, true, false);
    }
}

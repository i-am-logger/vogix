// A dashed rectangular border — the HUD's ALERT/ACTIVE frame state
// (hairline solid is the resting state; dashed marks something live).
import QtQuick

Canvas {
    id: root

    property color color: "#ff00ff"
    property real dashLength: 4
    property real gapLength: 3
    property real borderWidth: 1

    anchors.fill: parent

    onColorChanged: requestPaint()
    onWidthChanged: requestPaint()
    onHeightChanged: requestPaint()

    onPaint: {
        const ctx = getContext("2d");
        ctx.clearRect(0, 0, width, height);
        ctx.strokeStyle = root.color;
        ctx.lineWidth = root.borderWidth;
        ctx.setLineDash([root.dashLength, root.gapLength]);
        const half = root.borderWidth / 2;
        ctx.strokeRect(half, half, width - root.borderWidth, height - root.borderWidth);
    }
}

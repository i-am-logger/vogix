// Network throughput cell: ▲upload ▼download, humanized, widths
// reserved so rate flapping never reflows the bar.
import QtQuick
import qs.Bar.widgets
import qs.Components
import qs.Services
import qs.Vogix

FrameCell {
    id: root

    title: "NET"
    padH: 8
    padV: 3

    function fmt(rate: real): string {
        if (rate >= 1024 * 1024)
            return (rate / (1024 * 1024)).toFixed(1) + "M";
        if (rate >= 1024)
            return String(Math.round(rate / 1024)).padStart(3, "0") + "K";
        return String(Math.round(rate)).padStart(3, "0") + "B";
    }

    Row {
        spacing: Metrics.unit

        NumericText {
            text: "▲" + root.fmt(SysStat.netTxRate)
            widestText: "▲999.9M"
            font.pixelSize: Metrics.bodySmall
            color: Tokens.color("bar", "muted")
            anchors.verticalCenter: parent.verticalCenter
        }

        NumericText {
            text: "▼" + root.fmt(SysStat.netRxRate)
            widestText: "▼999.9M"
            font.pixelSize: Metrics.bodySmall
            color: Tokens.color("bar", "foreground")
            anchors.verticalCenter: parent.verticalCenter
        }
    }
}

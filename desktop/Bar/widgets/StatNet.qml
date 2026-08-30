// Network throughput cell: ▼download ▲upload, humanized, widths
// reserved so rate flapping never reflows the bar.
import QtQuick
import qs.Bar.widgets
import qs.Components
import qs.Services
import qs.Vogix

Rectangle {
    id: root

    function fmt(rate: real): string {
        if (rate >= 1024 * 1024)
            return (rate / (1024 * 1024)).toFixed(1) + "M";
        if (rate >= 1024)
            return Math.round(rate / 1024) + "K";
        return Math.round(rate) + "B";
    }

    implicitWidth: row.implicitWidth + Metrics.unit * 3
    implicitHeight: Metrics.chip
    color: "transparent"
    border.width: 1
    border.color: Tokens.color("meter", "frame")

    Row {
        id: row
        anchors.centerIn: parent
        spacing: Metrics.unit

        HudLabel {
            text: "NET"
            color: Tokens.color("meter", "label")
            anchors.verticalCenter: parent.verticalCenter
        }

        NumericText {
            text: "▼" + root.fmt(SysStat.netRxRate)
            widestText: "▼999.9M"
            font.pixelSize: Metrics.bodySmall
            color: Tokens.color("bar", "foreground")
            anchors.verticalCenter: parent.verticalCenter
        }

        NumericText {
            text: "▲" + root.fmt(SysStat.netTxRate)
            widestText: "▲999.9M"
            font.pixelSize: Metrics.bodySmall
            color: Tokens.color("bar", "muted")
            anchors.verticalCenter: parent.verticalCenter
        }
    }
}

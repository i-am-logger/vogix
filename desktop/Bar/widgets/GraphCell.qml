// A rail graph panel: framed break-title, sparkline, current value —
// the Flight Deck CPU/MEM/NET stack.
import QtQuick
import qs.Bar.widgets
import qs.Components
import qs.Vogix

FrameCell {
    id: root

    property alias values: line.values
    property alias secondary: line.secondary
    property alias lineColor: line.lineColor
    property alias secondaryColor: line.secondaryColor
    property string valueText: ""

    padH: 6
    padV: 5

    Column {
        spacing: 3

        Sparkline {
            id: line
            width: Metrics.body * 2.6
            height: Metrics.body * 1.75
            lineColor: Tokens.color("bar", "accent")
            secondaryColor: Tokens.color("meter", "mid")
            anchors.horizontalCenter: parent.horizontalCenter
        }

        BarText {
            text: root.valueText
            font.pixelSize: Metrics.caption
            color: Tokens.color("meter", "value")
            anchors.horizontalCenter: parent.horizontalCenter
        }
    }
}

// A rail graph cell: micro label over a sparkline, sized for the
// vertical rail's width.
import QtQuick
import qs.Components
import qs.Vogix

Column {
    id: root

    property string label: ""
    property alias values: line.values
    property alias secondary: line.secondary
    property alias lineColor: line.lineColor
    property alias secondaryColor: line.secondaryColor

    spacing: 2

    HudLabel {
        text: root.label
        font.pixelSize: Metrics.micro
        color: Tokens.color("meter", "label")
        anchors.horizontalCenter: parent.horizontalCenter
    }

    Sparkline {
        id: line
        width: Metrics.body * 2.25
        height: Metrics.body * 1.5
        lineColor: Tokens.color("bar", "accent")
        secondaryColor: Tokens.color("meter", "label")
    }
}

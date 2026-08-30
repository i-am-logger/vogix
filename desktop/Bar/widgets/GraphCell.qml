// A history graph cell for the bottom bar: framed, sparkline first,
// optional value beside it. A graph sitting next to its stat cell keeps
// title and value EMPTY — the adjacency binds them and the stat cell
// already carries the number; standalone graphs (I/O, GPU) carry both.
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

    padH: 8
    padV: 3

    Row {
        spacing: Metrics.unit * 2

        Sparkline {
            id: line
            width: Metrics.body * 3.5
            height: Metrics.body
            lineColor: Tokens.color("bar", "accent")
            secondaryColor: Tokens.color("meter", "mid")
            anchors.verticalCenter: parent.verticalCenter
        }

        NumericText {
            visible: root.valueText !== ""
            text: root.valueText
            widestText: "999.9M"
            font.pixelSize: Metrics.caption
            color: Tokens.color("meter", "value")
            anchors.verticalCenter: parent.verticalCenter
        }
    }
}

// A history graph cell: framed, sparkline first, optional value beside
// it on a horizontal bar — stacked below it on a rail. A graph sitting
// next to its stat cell keeps title and value EMPTY (the adjacency
// binds them and the stat cell carries the number); standalone graphs
// (I/O, GPU) carry both.
import QtQuick
import qs.Bar.widgets
import qs.Components
import qs.Vogix

FrameCell {
    id: root

    property BarAxis axis: null
    readonly property bool vertical: axis?.vertical ?? false

    property alias values: line.values
    property alias secondary: line.secondary
    property alias lineColor: line.lineColor
    property alias secondaryColor: line.secondaryColor
    property string valueText: ""

    padH: vertical ? 5 : 8
    padV: 3

    Grid {
        columns: root.vertical ? 1 : 2
        spacing: root.vertical ? Metrics.unit : Metrics.unit * 2
        verticalItemAlignment: Grid.AlignVCenter
        horizontalItemAlignment: Grid.AlignHCenter

        Sparkline {
            id: line
            width: root.vertical ? Metrics.body * 2.75 : Metrics.body * 3.5
            height: root.vertical ? Metrics.body * 1.5 : Metrics.body
            lineColor: Tokens.color("bar", "accent")
            secondaryColor: Tokens.color("meter", "mid")
        }

        NumericText {
            visible: root.valueText !== ""
            text: root.valueText
            widestText: "999.9M"
            font.pixelSize: Metrics.caption
            color: Tokens.color("meter", "value")
        }
    }
}
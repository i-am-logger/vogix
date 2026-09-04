// One Flight Deck stat cell: the framed break-title box with a
// width-reserved, zero-padded value (042% — digits never reflow the
// bar), optionally led by a small segmented meter (bar THEN number —
// the capacity gauges read at a glance, the number confirms). The value
// color carries the threshold state.
import QtQuick
import qs.Bar.widgets
import qs.Components
import qs.Vogix

FrameCell {
    id: root

    property BarAxis axis: null
    readonly property bool vertical: axis?.vertical ?? false

    property alias value: valueText.text
    property alias widestValue: valueText.widestText
    property color valueColor: Tokens.color("bar", "foreground")

    // 0..1 shows a meter before the number; keep at -1 for number-only.
    property real meterValue: -1
    property real meterWarnAt: 0.7
    property real meterDangerAt: 0.85

    // Optional history trace, drawn inside the SAME box below the value —
    // on the rail each metric is ONE instrument panel, like VU and MIC.
    property alias traceValues: trace.values
    property alias traceSecondary: trace.secondary
    property alias traceColor: trace.lineColor

    // A small secondary readout under the value — the mount cells put
    // their filesystem's live I/O here.
    property string subText: ""

    padH: vertical ? 6 : 8
    padV: vertical ? 6 : 3

    // Bar-then-number reads left-to-right on a horizontal bar; a rail
    // stacks a RISING gauge column (VU-panel proportions) over the value
    // over the trace, all centered.
    Grid {
        columns: root.vertical ? 1 : 4
        spacing: root.vertical ? Metrics.unit : Metrics.unit * 2
        verticalItemAlignment: Grid.AlignVCenter
        horizontalItemAlignment: Grid.AlignHCenter

        SegmentedMeter {
            visible: root.meterValue >= 0
            vertical: root.vertical
            width: root.vertical ? Math.round(Metrics.body * 0.75) : Metrics.body * 3
            height: root.vertical ? Metrics.body * 3.5 : Math.round(Metrics.body * 0.55)
            value: Math.max(0, root.meterValue)
            warnAt: root.meterWarnAt
            dangerAt: root.meterDangerAt
            low: Tokens.color("meter", "low")
            mid: Tokens.color("meter", "mid")
            high: Tokens.color("meter", "high")
            unlit: Tokens.color("meter", "unlit")
            capColor: Tokens.color("meter", "cap")
        }

        NumericText {
            id: valueText
            color: root.valueColor
            font.pixelSize: Metrics.bodySmall
        }

        NumericText {
            visible: root.subText !== ""
            text: root.subText
            widestText: "⇅999.9M"
            font.pixelSize: Metrics.micro
            color: Tokens.color("meter", "label")
        }

        Sparkline {
            id: trace
            visible: values.length > 0
            width: root.vertical ? Metrics.body * 2.5 : Metrics.body * 3.5
            height: root.vertical ? Metrics.body * 1.25 : Metrics.body
            lineColor: Tokens.color("bar", "accent")
            secondaryColor: Tokens.color("meter", "mid")
        }
    }
}

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

    property alias value: valueText.text
    property alias widestValue: valueText.widestText
    property color valueColor: Tokens.color("bar", "foreground")

    // 0..1 shows a meter before the number; keep at -1 for number-only.
    property real meterValue: -1
    property real meterWarnAt: 0.7
    property real meterDangerAt: 0.85

    padH: 8
    padV: 3

    Row {
        spacing: Metrics.unit * 2

        SegmentedMeter {
            visible: root.meterValue >= 0
            width: Metrics.body * 3
            height: Math.round(Metrics.body * 0.55)
            value: Math.max(0, root.meterValue)
            warnAt: root.meterWarnAt
            dangerAt: root.meterDangerAt
            low: Tokens.color("meter", "low")
            mid: Tokens.color("meter", "mid")
            high: Tokens.color("meter", "high")
            unlit: Tokens.color("meter", "unlit")
            capColor: Tokens.color("meter", "cap")
            anchors.verticalCenter: parent.verticalCenter
        }

        NumericText {
            id: valueText
            color: root.valueColor
            font.pixelSize: Metrics.bodySmall
            anchors.verticalCenter: parent.verticalCenter
        }
    }
}

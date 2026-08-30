// One Flight Deck stat cell: the framed break-title box with a
// width-reserved, zero-padded value (042% — digits never reflow the
// bar). The value color carries the threshold state.
import QtQuick
import qs.Bar.widgets
import qs.Components
import qs.Vogix

FrameCell {
    id: root

    property alias value: valueText.text
    property alias widestValue: valueText.widestText
    property color valueColor: Tokens.color("bar", "foreground")

    padH: 8
    padV: 3

    NumericText {
        id: valueText
        color: root.valueColor
        font.pixelSize: Metrics.bodySmall
    }
}

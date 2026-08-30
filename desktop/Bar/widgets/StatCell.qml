// One Flight Deck stat cell: hairline square frame, terse uppercase
// label, width-reserved value (no jitter as digits change). The value
// color carries the threshold state.
import QtQuick
import qs.Components
import qs.Vogix

Rectangle {
    id: root

    property string label: ""
    property alias value: valueText.text
    property alias widestValue: valueText.widestText
    property color valueColor: Tokens.color("bar", "foreground")

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
            text: root.label
            color: Tokens.color("meter", "label")
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

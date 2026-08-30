// Keyboard layout chip (EN/HE/…): the vogix-input device's active xkb
// layout. Click cycles to the next layout.
import QtQuick
import qs.Bar.widgets
import qs.Vogix
import qs.Services

Rectangle {
    id: root

    implicitWidth: label.implicitWidth + Metrics.unit * 3
    implicitHeight: Metrics.chip
    color: "transparent"
    border.width: 1
    border.color: Tokens.color("meter", "frame")

    BarText {
        id: label
        anchors.centerIn: parent
        text: KbLayout.label
        font.pixelSize: Metrics.caption
        font.letterSpacing: 1
        color: Tokens.color("bar", "foreground")
    }

    MouseArea {
        anchors.fill: parent
        onClicked: KbLayout.next()
    }
}

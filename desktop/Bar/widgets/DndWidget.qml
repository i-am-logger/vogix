// Do-not-disturb indicator: only shown while DND is on; click lifts it.
import QtQuick
import qs.Bar.widgets
import qs.Services
import qs.Vogix

BarText {
    visible: Notifs.dnd
    text: "󰂛"
    color: Tokens.color("bar", "accent")

    MouseArea {
        anchors.fill: parent
        onClicked: Notifs.setDnd(false)
    }
}

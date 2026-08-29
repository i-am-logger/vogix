// Small toggles that only appear while active: night light and stay-awake.
// Click turns the thing off (turning on goes through the menu or verbs).
import QtQuick
import QtQuick.Layouts
import qs.Bar.widgets
import qs.Services
import qs.Vogix

RowLayout {
    spacing: 8

    BarText {
        visible: Nightlight.on
        text: "󱩌"
        color: Tokens.color("bar", "accent")

        MouseArea {
            anchors.fill: parent
            onClicked: Nightlight.set(false)
        }
    }

    BarText {
        visible: StayAwake.on
        text: "󰅶"
        color: Tokens.color("bar", "accent")

        MouseArea {
            anchors.fill: parent
            onClicked: StayAwake.set(false)
        }
    }
}

// Small indicators that only appear while active: night light and
// stay-awake (a click turns them off; turning on goes through the menu or
// verbs), and the count of pending reminders.
import QtQuick
import QtQuick.Layouts
import qs.Bar.widgets
import qs.Services
import qs.Vogix

GridLayout {
    id: root

    property BarAxis axis: null

    flow: (axis?.vertical ?? false) ? GridLayout.TopToBottom : GridLayout.LeftToRight
    rowSpacing: 8
    columnSpacing: 8

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

    BarText {
        visible: Reminders.entries.length > 0
        text: "󰀠 " + Reminders.entries.length
        color: Tokens.color("bar", "accent")
    }
}

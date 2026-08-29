pragma ComponentBehavior: Bound
// Hyprland workspaces: focused gets the accent, urgent the urgent token.
// Clicks dispatch through Quickshell.Hyprland, which speaks whichever
// config engine the compositor runs (Lua-aware since 0.3.0).
import QtQuick
import QtQuick.Layouts
import Quickshell.Hyprland
import qs.Bar.widgets
import qs.Vogix

RowLayout {
    spacing: 4

    Repeater {
        model: Hyprland.workspaces.values

        Rectangle {
            id: ws

            required property var modelData

            implicitWidth: Math.max(22, label.implicitWidth + 10)
            implicitHeight: 22
            radius: 4
            color: ws.modelData.focused ? Tokens.color("bar", "accent") : "transparent"
            border.width: ws.modelData.urgent ? 1 : 0
            border.color: Tokens.color("bar", "urgent")

            BarText {
                id: label
                anchors.centerIn: parent
                text: ws.modelData.name
                color: ws.modelData.focused
                    ? Tokens.color("bar", "background")
                    : Tokens.color("bar", "foreground")
            }

            MouseArea {
                anchors.fill: parent
                onClicked: Hyprland.dispatch("workspace " + ws.modelData.id)
            }
        }
    }
}

pragma ComponentBehavior: Bound
// Hyprland workspaces: focused gets the accent, urgent the urgent token.
// Clicks dispatch through Quickshell.Hyprland, which speaks whichever
// config engine the compositor runs (Lua-aware since 0.3.0).
import QtQuick
import QtQuick.Layouts
import Quickshell.Hyprland
import qs.Bar.widgets
import qs.Vogix

GridLayout {
    id: root

    property BarAxis axis: null

    flow: (axis?.vertical ?? false) ? GridLayout.TopToBottom : GridLayout.LeftToRight
    rowSpacing: 4
    columnSpacing: 4

    Repeater {
        model: Hyprland.workspaces.values

        Rectangle {
            id: ws

            required property var modelData

            // Square HUD blocks: chip-sized, hairline-framed, filled when
            // focused.
            implicitWidth: Math.max(Metrics.chip, label.implicitWidth + 10)
            implicitHeight: Metrics.chip
            color: ws.modelData.focused ? Tokens.color("bar", "accent") : "transparent"
            border.width: 1
            border.color: ws.modelData.urgent
                ? Tokens.color("bar", "urgent")
                : Tokens.color("meter", "frame")

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

pragma ComponentBehavior: Bound
// Hyprland workspaces as square HUD blocks: focused fills with the
// accent, urgent frames in the urgent token. Horizontal bars get the
// framed WS cell; the vertical rail gets the bare stacked column, like
// the Flight Deck board. Clicks dispatch through Quickshell.Hyprland.
import QtQuick
import Quickshell.Hyprland
import qs.Bar.widgets
import qs.Vogix

Loader {
    id: root

    property BarAxis axis: null
    readonly property bool vertical: axis?.vertical ?? false

    component WsBox: Rectangle {
        id: ws

        required property var modelData

        implicitWidth: Math.max(Metrics.body + 6, label.implicitWidth + 10)
        implicitHeight: Metrics.body + 6
        color: ws.modelData.focused ? Tokens.color("bar", "accent") : "transparent"
        border.width: 1
        border.color: ws.modelData.urgent
            ? Tokens.color("bar", "urgent")
            : (ws.modelData.focused ? Tokens.color("bar", "accent") : Tokens.color("meter", "frame"))

        BarText {
            id: label
            anchors.centerIn: parent
            text: ws.modelData.name
            font.pixelSize: Metrics.caption
            font.bold: ws.modelData.focused
            color: ws.modelData.focused
                ? Tokens.color("bar", "background")
                : Tokens.color("bar", "foreground")
        }

        MouseArea {
            anchors.fill: parent
            onClicked: Hyprland.dispatch("workspace " + ws.modelData.id)
        }
    }

    sourceComponent: vertical ? railForm : cellForm

    Component {
        id: cellForm

        FrameCell {
            title: "WS"
            padH: 6
            padV: 3

            Row {
                spacing: 3

                Repeater {
                    model: Hyprland.workspaces.values

                    WsBox {}
                }
            }
        }
    }

    Component {
        id: railForm

        Column {
            spacing: 6

            Repeater {
                model: Hyprland.workspaces.values

                WsBox {}
            }
        }
    }
}

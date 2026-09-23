pragma ComponentBehavior: Bound
// Hyprland workspaces as square HUD blocks: focused fills with the
// accent, urgent frames in the urgent token. Horizontal bars get the
// framed WS cell; the vertical rail gets the bare stacked column, like
// the Flight Deck board. A click focuses the workspace through
// HyprlandWorkspace.activate(), which writes the dispatch in the dialect
// of the compositor's config engine (hyprlang or Lua).
//
// A special workspace (Hyprland names it "special:<name>") reads by its
// own name, in the muted color, so a scratchpad such as the console does
// not read as a place in the numbered run. A block on a rail is never
// wider than the rail leaves room for: a longer name is cut short.
import QtQuick
import Quickshell.Hyprland
import qs.Bar.widgets
import qs.Vogix

Loader {
    id: root

    property BarAxis axis: null
    readonly property bool vertical: axis?.vertical ?? false
    // The widest a block may be: the rail less a unit of margin each side.
    readonly property real maxBoxWidth: vertical ? (axis?.thickness ?? 0) - Metrics.unit * 2 : Infinity

    component WsBox: Rectangle {
        id: ws

        required property HyprlandWorkspace modelData

        // The bar's limit on a block's width (maxBoxWidth).
        property real maxWidth: Infinity
        readonly property bool special: ws.modelData.name.startsWith("special:")

        implicitWidth: Math.min(ws.maxWidth, Math.max(Metrics.body + 6, label.implicitWidth + 10))
        implicitHeight: Metrics.body + 6
        color: ws.modelData.focused ? Tokens.color("bar", "accent") : "transparent"
        border.width: 1
        border.color: ws.modelData.urgent
            ? Tokens.color("bar", "urgent")
            : (ws.modelData.focused ? Tokens.color("bar", "accent") : Tokens.color("meter", "frame"))

        BarText {
            id: label
            anchors.centerIn: parent
            width: Math.min(label.implicitWidth, ws.width - 10)
            elide: Text.ElideRight
            text: ws.special ? ws.modelData.name.slice("special:".length) : ws.modelData.name
            font.pixelSize: Metrics.caption
            font.bold: ws.modelData.focused
            color: ws.modelData.focused ? Tokens.color("bar", "background")
                : ws.special ? Tokens.color("bar", "muted")
                : Tokens.color("bar", "foreground")
        }

        MouseArea {
            anchors.fill: parent
            onClicked: ws.modelData.activate()
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

                    WsBox {
                        maxWidth: root.maxBoxWidth
                    }
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

                WsBox {
                    maxWidth: root.maxBoxWidth
                }
            }
        }
    }
}

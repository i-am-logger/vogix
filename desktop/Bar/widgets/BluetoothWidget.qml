// Bluetooth: hidden without an adapter; accent while something is
// connected; click opens the bluetooth panel.
import QtQuick
import Quickshell.Bluetooth
import qs.Bar.widgets
import qs.Services
import qs.Vogix

BarText {
    readonly property var adapter: Bluetooth.defaultAdapter
    readonly property bool anyConnected:
        Bluetooth.devices.values.some(d => d.connected)

    visible: adapter !== null
    text: (adapter?.enabled ?? false) ? (anyConnected ? "󰂱" : "󰂯") : "󰂲"
    color: anyConnected
        ? Tokens.color("bar", "accent")
        : Tokens.color("bar", "foreground")

    MouseArea {
        anchors.fill: parent
        onClicked: Panels.toggle("bluetooth")
    }
}

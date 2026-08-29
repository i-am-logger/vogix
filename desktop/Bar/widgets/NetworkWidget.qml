// Primary network state: ethernet/wifi icon, muted when disconnected;
// click opens the network panel.
import QtQuick
import Quickshell.Networking
import qs.Bar.widgets
import qs.Services
import qs.Vogix

BarText {
    readonly property var devices: Networking.devices.values
    readonly property var connectedDev: devices.find(d => d.connected) ?? null

    text: {
        if (!connectedDev)
            return "󰤮";
        return connectedDev.type === DeviceType.Wifi ? "󰤨" : "󰈀";
    }
    color: connectedDev
        ? Tokens.color("bar", "foreground")
        : Tokens.color("bar", "muted")

    MouseArea {
        anchors.fill: parent
        onClicked: Panels.toggle("network")
    }
}

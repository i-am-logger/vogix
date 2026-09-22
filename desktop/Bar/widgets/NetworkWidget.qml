// Primary network state: ethernet/wifi icon, muted when disconnected, a
// warning-colored network-off glyph when the shell has no NetworkManager
// backend (so an unknown state never reads as "disconnected"); click
// opens the network panel.
import QtQuick
import Quickshell.Networking
import qs.Bar.widgets
import qs.Services
import qs.Vogix

BarText {
    readonly property var devices: Networking.devices.values
    readonly property var connectedDev: devices.find(d => d.connected) ?? null

    text: {
        if (!NetworkBackend.attached)
            return "󰲛";
        if (!connectedDev)
            return "󰤮";
        return connectedDev.type === DeviceType.Wifi ? "󰤨" : "󰈀";
    }
    color: !NetworkBackend.attached ? Tokens.color("meter", "mid")
        : connectedDev ? Tokens.color("bar", "foreground")
        : Tokens.color("bar", "muted")

    MouseArea {
        anchors.fill: parent
        onClicked: Panels.toggle("network")
    }
}

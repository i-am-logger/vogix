// Tailscale: click opens the peers panel.
import QtQuick
import qs.Bar.widgets
import qs.Services
import qs.Vogix

BarText {
    text: "󰖂"
    color: Tokens.color("bar", "muted")

    MouseArea {
        anchors.fill: parent
        onClicked: Panels.toggle("tailscale")
    }
}

// Tailnet link glyph for the status column: success-colored while
// connected, warning while connecting or offline, muted while stopped,
// logged out or without its daemon; absent on hosts without tailscale.
// Click opens the tailnet panel (connection time, peers).
import QtQuick
import qs.Bar.widgets
import qs.Services
import qs.Vogix
import "../../Services/lib/tailnet.js" as Tailnet

BarText {
    visible: Tailscale.link !== Tailnet.Link.Absent
    text: "󰖂"
    color: {
        switch (Tailscale.link) {
        case Tailnet.Link.Connected:
            return Tokens.color("meter", "low");
        case Tailnet.Link.Connecting:
        case Tailnet.Link.Offline:
            return Tokens.color("meter", "mid");
        default:
            return Tokens.color("bar", "muted");
        }
    }

    MouseArea {
        anchors.fill: parent
        onClicked: Panels.toggle("tailscale")
    }
}

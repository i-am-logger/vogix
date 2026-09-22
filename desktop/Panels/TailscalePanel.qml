pragma ComponentBehavior: Bound
// Tailnet state from the Tailscale service: link state and connection
// time, this node, then peers, online first. Opening the panel takes a
// fresh sample.
import QtQuick
import QtQuick.Layouts
import Quickshell
import qs.Panels
import qs.Services
import qs.Vogix

ColumnLayout {
    id: root

    Component.onCompleted: Tailscale.refresh()

    spacing: 10

    SystemClock {
        id: clock
        precision: SystemClock.Minutes
    }

    PanelLabel {
        text: "Tailscale"
        font.bold: true
        color: Tokens.color("popup", "accent")
    }

    // "connected 3h12m · 4/7 peers online"; "≥" marks a connection the
    // shell found already up.
    PanelLabel {
        Layout.fillWidth: true
        text: Tailscale.online
            ? "connected " + Tailscale.sinceText(clock.date.getTime())
                + " · " + Tailscale.peersOnline + "/" + Tailscale.peersTotal + " peers online"
            : Tailscale.stateText()
        color: Tailscale.online ? Tokens.color("popup", "foreground") : Tokens.color("popup", "muted")
    }

    PanelLabel {
        visible: Tailscale.selfName !== ""
        Layout.fillWidth: true
        text: Tailscale.selfName + "  " + Tailscale.selfIp
    }

    ListView {
        Layout.fillWidth: true
        Layout.fillHeight: true
        Layout.minimumHeight: 180
        clip: true
        spacing: 2
        model: Tailscale.peers

        delegate: RowLayout {
            id: row

            required property var modelData

            width: ListView.view.width

            PanelLabel {
                Layout.fillWidth: true
                text: (row.modelData.online ? "󰄴 " : "󰄰 ") + row.modelData.name
            }

            PanelLabel {
                text: row.modelData.ip
                color: Tokens.color("popup", "muted")
            }
        }
    }
}

// Tailscale link: glyph lights accent while connected, with the
// connected-duration readout beside it. Click opens the peers panel.
// (The MouseArea must wrap the Row from outside — anchors on a direct
// Row child break the Row's own layout.)
import QtQuick
import Quickshell
import qs.Bar.widgets
import qs.Services
import qs.Vogix

Item {
    implicitWidth: row.implicitWidth
    implicitHeight: row.implicitHeight

    SystemClock {
        id: clock
        precision: SystemClock.Minutes
    }

    Row {
        id: row
        spacing: Metrics.unit

        BarText {
            text: "󰖂"
            color: Tailscale.online ? Tokens.color("bar", "accent") : Tokens.color("bar", "muted")
        }

        BarText {
            visible: Tailscale.online
            text: Tailscale.sinceText(clock.date.getTime())
            font.pixelSize: Metrics.caption
            color: Tokens.color("bar", "muted")
        }
    }

    MouseArea {
        anchors.fill: parent
        onClicked: Panels.toggle("tailscale")
    }
}

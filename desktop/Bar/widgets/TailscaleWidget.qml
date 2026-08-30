// The TS cell: peers online/total and how long the link has been up
// ("4/7 12d"). Title lights success while connected. Click opens the
// peers panel.
import QtQuick
import Quickshell
import qs.Bar.widgets
import qs.Services
import qs.Vogix

FrameCell {
    title: "TS"
    titleColor: Tailscale.online ? Tokens.color("meter", "low") : Tokens.color("meter", "label")
    padH: 8
    padV: 3

    SystemClock {
        id: clock
        precision: SystemClock.Minutes
    }

    BarText {
        text: Tailscale.online
            ? Tailscale.peersOnline + "/" + Tailscale.peersTotal + " " + Tailscale.sinceText(clock.date.getTime()).toUpperCase()
            : "OFF"
        font.pixelSize: Metrics.bodySmall
        color: Tailscale.online ? Tokens.color("bar", "foreground") : Tokens.color("bar", "muted")
    }

    MouseArea {
        anchors.fill: parent
        onClicked: Panels.toggle("tailscale")
    }
}

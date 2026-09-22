// The TS cell: peers online/total and how long the current tailnet
// connection has lasted ("4/7 12D"; "≥" marks a connection the shell found
// already up, so its true start is earlier). Title lights success while
// connected. Click opens the tailnet panel.
import QtQuick
import Quickshell
import qs.Bar.widgets
import qs.Services
import qs.Vogix

FrameCell {
    id: root

    property BarAxis axis: null

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

    interactive: true
    onClicked: root.axis?.togglePanel("tailscale", root) ?? Panels.toggle("tailscale")
}

// The THEME cell: name/variant with stepper arrows — ◂ darker,
// ▸ lighter, through `vogix theme set -v`, the same verb the keybindings
// use, so the palette flip recolors this very cell.
import QtQuick
import Quickshell
import qs.Bar.widgets
import qs.Vogix

FrameCell {
    title: "THEME"
    padH: 10
    padV: 3

    Row {
        spacing: Metrics.unit * 2

        BarText {
            text: "◂"
            color: darkerArea.containsMouse ? Tokens.color("bar", "accent") : Tokens.color("bar", "muted")
            anchors.verticalCenter: parent.verticalCenter

            MouseArea {
                id: darkerArea
                anchors.fill: parent
                anchors.margins: -3
                hoverEnabled: true
                onClicked: Quickshell.execDetached(["vogix", "theme", "set", "-v", "darker"])
            }
        }

        BarText {
            text: Theme.name + (Theme.variant !== "" ? "/" + Theme.variant : "")
            font.pixelSize: Metrics.bodySmall
            color: Tokens.color("bar", "foreground")
            anchors.verticalCenter: parent.verticalCenter
        }

        BarText {
            text: "▸"
            color: lighterArea.containsMouse ? Tokens.color("bar", "accent") : Tokens.color("bar", "muted")
            anchors.verticalCenter: parent.verticalCenter

            MouseArea {
                id: lighterArea
                anchors.fill: parent
                anchors.margins: -3
                hoverEnabled: true
                onClicked: Quickshell.execDetached(["vogix", "theme", "set", "-v", "lighter"])
            }
        }
    }
}

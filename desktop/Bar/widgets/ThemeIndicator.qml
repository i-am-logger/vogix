// Theme readout with variant steppers: ◂ steps the theme darker,
// ▸ lighter — `vogix theme set -v darker|lighter`, the same verb the
// keybindings use, so the palette flip recolors this very widget.
import QtQuick
import Quickshell
import qs.Bar.widgets
import qs.Vogix

Row {
    spacing: Metrics.unit

    BarText {
        text: "◂"
        color: darkerArea.containsMouse ? Tokens.color("bar", "accent") : Tokens.color("bar", "muted")

        MouseArea {
            id: darkerArea
            anchors.fill: parent
            anchors.margins: -2
            hoverEnabled: true
            onClicked: Quickshell.execDetached(["vogix", "theme", "set", "-v", "darker"])
        }
    }

    BarText {
        text: Theme.name + (Theme.variant !== "" ? "/" + Theme.variant : "")
        color: Tokens.color("bar", "muted")
    }

    BarText {
        text: "▸"
        color: lighterArea.containsMouse ? Tokens.color("bar", "accent") : Tokens.color("bar", "muted")

        MouseArea {
            id: lighterArea
            anchors.fill: parent
            anchors.margins: -2
            hoverEnabled: true
            onClicked: Quickshell.execDetached(["vogix", "theme", "set", "-v", "lighter"])
        }
    }
}

// The on-screen display: a small centered-bottom flash for volume/brightness
// and friends, driven entirely by qs.Services.Osd state.
import QtQuick
import Quickshell
import Quickshell.Hyprland
import qs.Services
import qs.Vogix

PanelWindow {
    id: root

    screen: {
        const name = Hyprland.focusedMonitor?.name ?? "";
        return Quickshell.screens.find(s => s.name === name) ?? Quickshell.screens[0] ?? null;
    }

    visible: Osd.visible
    anchors.bottom: true
    margins.bottom: 96
    implicitWidth: 280
    implicitHeight: 64
    color: "transparent"
    exclusionMode: ExclusionMode.Ignore

    Rectangle {
        anchors.fill: parent
        radius: 10
        color: Tokens.color("osd", "background")

        Column {
            anchors.centerIn: parent
            width: parent.width - 40
            spacing: 8

            Text {
                width: parent.width
                horizontalAlignment: Text.AlignHCenter
                text: Osd.message !== "" ? Osd.message
                    : (Osd.kind + (Osd.muted ? " muted"
                        : (Osd.value >= 0 ? " " + Math.round(Osd.value * 100) + "%" : "")))
                color: Osd.muted
                    ? Tokens.color("osd", "muted")
                    : Tokens.color("osd", "foreground")
                font.family: Config.fontFamily
                font.pixelSize: Metrics.body
            }

            Rectangle {
                visible: Osd.value >= 0
                width: parent.width
                height: 6
                radius: 3
                color: Tokens.color("osd", "muted")

                Rectangle {
                    width: parent.width * Math.min(1, Math.max(0, Osd.value))
                    height: parent.height
                    radius: 3
                    color: Osd.muted
                        ? Tokens.color("osd", "muted")
                        : Tokens.color("osd", "accent")
                }
            }
        }
    }
}

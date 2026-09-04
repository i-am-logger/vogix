// The root menu button — the touch/pointer way into `vogix desktop menu`.
import QtQuick
import qs.Bar.widgets
import qs.Services
import qs.Vogix

BarText {
    text: "󰍜"
    font.pixelSize: Metrics.display
    color: Tokens.color("bar", "muted")

    MouseArea {
        anchors.fill: parent
        onClicked: Launcher.openMenu("")
    }
}

// Power menu button for the rail.
import QtQuick
import qs.Bar.widgets
import qs.Services
import qs.Vogix

BarText {
    text: "󰐥"
    color: Tokens.color("bar", "muted")

    MouseArea {
        anchors.fill: parent
        onClicked: Power.show()
    }
}

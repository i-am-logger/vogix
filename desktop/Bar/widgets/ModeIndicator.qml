// The input engine's current mode (the mode-visibility surface the border
// color already carries; the bar is its second reader). The root mode stays
// muted; any other mode gets the accent.
import QtQuick
import qs.Bar.widgets
import qs.Vogix

Rectangle {
    implicitWidth: label.implicitWidth + 14
    implicitHeight: 22
    radius: 4
    color: Mode.mode === "app" ? "transparent" : Tokens.color("bar", "accent")

    BarText {
        id: label
        anchors.centerIn: parent
        text: Mode.mode
        color: Mode.mode === "app"
            ? Tokens.color("bar", "muted")
            : Tokens.color("bar", "background")
    }
}

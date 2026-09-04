// The LANG cell: every configured layout, the active one lit bright and
// bold, the rest dim. Click cycles.
//
// CAPS sits alongside because Alt+CapsLock is what cycles the layout: pressing
// it with caps already latched looks identical to pressing it without, and the
// difference only shows up in what you type next.
import QtQuick
import qs.Bar.widgets
import qs.Services
import qs.Vogix

FrameCell {
    id: root

    title: "LANG"
    padH: 10
    padV: 3

    Row {
        spacing: Metrics.unit * 2

        Repeater {
            model: KbLayout.layouts.length > 0 ? KbLayout.layouts : [""]

            BarText {
                required property string modelData
                readonly property bool active: KbLayout.codeLabel(modelData) === KbLayout.label

                text: modelData === "" ? KbLayout.label : KbLayout.codeLabel(modelData)
                font.pixelSize: active ? Metrics.bodySmall : Metrics.caption
                font.bold: active
                color: active
                    ? Tokens.color("bar", "foreground")
                    : Tokens.color("bar", "muted")
                anchors.verticalCenter: parent.verticalCenter
            }
        }

        // Lit/dim rather than shown/hidden: a cell that changes width every
        // time caps is pressed shoves the rest of the bar sideways, and the
        // eye reads the movement before the letters.
        BarText {
            text: "CAPS"
            font.pixelSize: Metrics.caption
            font.bold: KbLayout.capsOn
            color: KbLayout.capsOn
                ? Tokens.color("bar", "foreground")
                : Tokens.color("bar", "muted")
            anchors.verticalCenter: parent.verticalCenter
        }
    }

    interactive: true
    onClicked: KbLayout.next()
}

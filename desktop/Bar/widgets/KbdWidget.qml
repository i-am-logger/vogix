// The LANG cell: every configured layout, the active one — by the index
// Hyprland reports — lit bright and bold, the rest dim. Click cycles.
//
// CAPS sits alongside because Alt+CapsLock is what cycles the layout: pressing
// it with caps already latched looks identical to pressing it without, and the
// difference only shows up in what you type next.
//
// A row on a horizontal bar. A rail is too narrow for the layouts and CAPS
// side by side, so there they stack.
import QtQuick
import QtQuick.Layouts
import qs.Bar.widgets
import qs.Services
import qs.Vogix

FrameCell {
    id: root

    property BarAxis axis: null
    readonly property bool vertical: axis?.vertical ?? false

    title: "LANG"
    padH: 10
    padV: 3

    GridLayout {
        flow: root.vertical ? GridLayout.TopToBottom : GridLayout.LeftToRight
        rowSpacing: Metrics.unit
        columnSpacing: Metrics.unit * 2

        Repeater {
            model: KbLayout.layouts.length > 0 ? KbLayout.layouts : [""]

            BarText {
                required property string modelData
                required property int index
                readonly property bool active: KbLayout.layouts.length > 0
                    && index === KbLayout.activeIndex

                Layout.alignment: Qt.AlignCenter
                text: modelData === "" ? "??" : KbLayout.codeLabel(modelData)
                font.pixelSize: active ? Metrics.bodySmall : Metrics.caption
                font.bold: active
                color: active
                    ? Tokens.color("bar", "foreground")
                    : Tokens.color("bar", "muted")
            }
        }

        // Lit/dim rather than shown/hidden: a cell that changes width every
        // time caps is pressed shoves the rest of the bar sideways, and the
        // eye reads the movement before the letters. Hidden only while the
        // state is unknown (no input engine, or no keyboard with a CapsLock
        // LED), which does not change per keypress.
        BarText {
            Layout.alignment: Qt.AlignCenter
            visible: KbLayout.capsKnown
            text: "CAPS"
            font.pixelSize: Metrics.caption
            font.bold: KbLayout.capsOn
            color: KbLayout.capsOn
                ? Tokens.color("bar", "foreground")
                : Tokens.color("bar", "muted")
        }
    }

    interactive: true
    onClicked: KbLayout.next()
}

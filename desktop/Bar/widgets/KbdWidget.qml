// The LANG cell: every configured layout, the active one lit bright and
// bold, the rest dim. Click cycles.
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
    }

    interactive: true
    onClicked: KbLayout.next()
}

// The input-device cell: which source the desktop is listening on, and
// whether that source is open at all. The glyph is the mute state and
// toggles it; the rest of the cell opens the device list. The title
// lights urgent while something is actually capturing — the same signal
// the MIC meter carries, so the two agree wherever they sit together.
// Reads on both bar orientations: two wrapped lines on a rail, one
// elided line on a horizontal bar.
import QtQuick
import qs.Bar.widgets
import qs.Services
import qs.Vogix

FrameCell {
    id: root

    property BarAxis axis: null
    readonly property bool vertical: axis?.vertical ?? false

    // On a rail the bar's own thickness is the budget; the fallback is
    // for the standalone case (gallery, preview) where no axis arrives.
    readonly property int labelWidth: root.vertical
        ? Math.max(Metrics.body * 2,
            Math.min(Metrics.body * 3,
                (root.axis?.thickness ?? Metrics.body * 4) - Metrics.unit * 6))
        : Metrics.body * 7

    title: "IN"
    titleColor: Privacy.micInUse ? Tokens.color("bar", "urgent") : Tokens.color("meter", "label")
    padH: 6
    padV: 4
    interactive: true
    onClicked: Panels.toggle("audio-in")

    Column {
        spacing: Metrics.unit

        BarText {
            text: Audio.micMuted ? "󰍭" : "󰍬"
            font.pixelSize: Metrics.bodySmall
            color: Audio.micMuted
                ? Tokens.color("bar", "muted")
                : Tokens.color("bar", "foreground")
            anchors.horizontalCenter: parent.horizontalCenter

            // Sized to the glyph, not to the cell's slot — a filling
            // child there feeds childrenRect back into the implicit size.
            MouseArea {
                anchors.fill: parent
                onClicked: Audio.toggleMic()
            }
        }

        BarText {
            width: root.labelWidth
            text: Audio.label(Audio.source) || "none"
            font.pixelSize: Metrics.micro
            font.letterSpacing: 0.5
            color: Tokens.color("bar", "muted")
            horizontalAlignment: Text.AlignHCenter
            wrapMode: Text.Wrap
            maximumLineCount: root.vertical ? 2 : 1
            elide: Text.ElideRight
        }
    }
}

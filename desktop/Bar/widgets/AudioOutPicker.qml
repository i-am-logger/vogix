// The output-device cell: which sink the desktop is playing through.
// Pipewire names run to whole sentences ("Family 17h/19h HD Audio
// Controller Analog Stereo"), so the label is width-capped and elides —
// the full name lives one click away in the audio-out panel. The glyph
// carries the sink's mute and toggles it; the rest of the cell opens the
// device list. Reads on both bar orientations: two wrapped lines on a
// rail, one elided line on a horizontal bar.
import QtQuick
import qs.Bar.widgets
import qs.Services
import qs.Vogix

FrameCell {
    id: root

    property BarAxis axis: null
    readonly property bool vertical: axis?.vertical ?? false
    readonly property var audio: Audio.sink?.audio ?? null
    readonly property bool muted: audio?.muted ?? false

    // On a rail the bar's own thickness is the budget; the fallback is
    // for the standalone case (gallery, preview) where no axis arrives.
    readonly property int labelWidth: root.vertical
        ? Math.max(Metrics.body * 2,
            Math.min(Metrics.body * 3,
                (root.axis?.thickness ?? Metrics.body * 4) - Metrics.unit * 6))
        : Metrics.body * 7

    title: "OUT"
    padH: 6
    padV: 4
    interactive: true
    onClicked: Panels.toggle("audio-out")

    Column {
        spacing: Metrics.unit

        BarText {
            text: root.muted ? "󰝟" : "󰕾"
            font.pixelSize: Metrics.bodySmall
            color: root.muted
                ? Tokens.color("bar", "muted")
                : Tokens.color("bar", "foreground")
            anchors.horizontalCenter: parent.horizontalCenter

            // Sized to the glyph, not to the cell's slot — a filling
            // child there feeds childrenRect back into the implicit size.
            MouseArea {
                anchors.fill: parent
                onClicked: {
                    if (root.audio)
                        root.audio.muted = !root.audio.muted;
                }
            }
        }

        BarText {
            width: root.labelWidth
            text: Audio.label(Audio.sink) || "none"
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

// The MEDIA cell: transport for the active Mpris player — previous,
// play/pause, next — and no track text. The MEDIA label lights while
// something plays; a control the player cannot perform right now stays
// unlit; the mouse wheel over the controls skips tracks. Hidden while no
// player is around.
import QtQuick
import qs.Bar.widgets
import qs.Services
import qs.Vogix

FrameCell {
    id: root

    readonly property bool playing: Media.active?.isPlaying ?? false

    // One transport glyph: accent under the pointer, unlit while its
    // action is unavailable.
    component Control: BarText {
        id: control

        required property bool available
        property color restColor: Tokens.color("bar", "muted")

        signal activated()

        color: !control.available ? Tokens.color("meter", "unlit")
            : (area.containsMouse ? Tokens.color("bar", "accent") : control.restColor)

        // Sized to the glyph, not to the cell's slot (see FrameCell).
        MouseArea {
            id: area
            anchors.fill: parent
            anchors.margins: -3
            hoverEnabled: true
            enabled: control.available
            onClicked: control.activated()
        }
    }

    visible: Media.active !== null
    title: "MEDIA"
    titleColor: root.playing ? Tokens.color("bar", "accent") : Tokens.color("meter", "label")
    padH: 10
    padV: 3

    Row {
        spacing: Metrics.unit * 3

        Control {
            text: "󰒮"
            available: Media.active?.canGoPrevious ?? false
            anchors.verticalCenter: parent.verticalCenter
            onActivated: Media.previous()
        }

        Control {
            text: root.playing ? "󰏤" : "󰐊"
            font.pixelSize: Metrics.subtitle
            available: Media.active?.canTogglePlaying ?? false
            restColor: root.playing ? Tokens.color("bar", "foreground") : Tokens.color("bar", "muted")
            anchors.verticalCenter: parent.verticalCenter
            onActivated: Media.playPause()
        }

        Control {
            text: "󰒭"
            available: Media.active?.canGoNext ?? false
            anchors.verticalCenter: parent.verticalCenter
            onActivated: Media.next()
        }

        // A mouse wheel only: a touchpad's stream of small deltas would
        // skip a run of tracks per swipe.
        WheelHandler {
            acceptedDevices: PointerDevice.Mouse
            onWheel: event => event.angleDelta.y > 0 ? Media.previous() : Media.next()
        }
    }
}

// Now playing (Mpris): click toggles play/pause, wheel skips tracks.
import QtQuick
import qs.Bar.widgets
import qs.Services
import qs.Vogix

BarText {
    visible: Media.active !== null
    elide: Text.ElideRight
    width: Math.min(implicitWidth, 260)
    text: ((Media.active?.isPlaying ?? false) ? "󰏤 " : "󰐊 ") + Media.title
    color: (Media.active?.isPlaying ?? false)
        ? Tokens.color("bar", "foreground")
        : Tokens.color("bar", "muted")

    MouseArea {
        anchors.fill: parent
        onClicked: Media.playPause()
        onWheel: wheel => wheel.angleDelta.y > 0 ? Media.previous() : Media.next()
    }
}

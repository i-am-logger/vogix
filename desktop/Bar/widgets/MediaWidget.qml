// Now playing (Mpris): click toggles play/pause, wheel skips tracks.
// Transport GLYPH only — the track title is deliberately not drawn here. It
// is unbounded text on a bar whose other occupant is a spectrum, so it set
// the whole left end's width from whatever happened to be playing.
import QtQuick
import qs.Bar.widgets
import qs.Services
import qs.Vogix

BarText {
    visible: Media.active !== null
    text: (Media.active?.isPlaying ?? false) ? "󰏤" : "󰐊"
    color: (Media.active?.isPlaying ?? false)
        ? Tokens.color("bar", "foreground")
        : Tokens.color("bar", "muted")

    MouseArea {
        anchors.fill: parent
        onClicked: Media.playPause()
        onWheel: wheel => wheel.angleDelta.y > 0 ? Media.previous() : Media.next()
    }
}

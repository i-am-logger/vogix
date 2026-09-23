pragma Singleton
// Mpris: the active player — the one actually playing; else the one that
// last played, while it is still around; else the first that can be
// controlled. A pause leaves the transport on the player it paused.
import QtQuick
import Quickshell
import Quickshell.Services.Mpris

Singleton {
    id: root

    readonly property var players: Mpris.players.values
    readonly property var playingPlayer: players.find(p => p.isPlaying) ?? null
    property var _last: null
    readonly property var active: playingPlayer
        ?? (players.includes(_last) ? _last : null)
        ?? players.find(p => p.canControl)
        ?? null

    // The player that last played.
    function _remember(): void {
        if (root.playingPlayer)
            root._last = root.playingPlayer;
    }

    onPlayingPlayerChanged: _remember()
    Component.onCompleted: _remember()

    function playPause(): void {
        if (root.active?.canTogglePlaying)
            root.active.togglePlaying();
    }

    function next(): void {
        if (root.active?.canGoNext)
            root.active.next();
    }

    function previous(): void {
        if (root.active?.canGoPrevious)
            root.active.previous();
    }
}

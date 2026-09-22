pragma Singleton
// Mpris: the active player — the one actually playing, else the first that
// can be controlled.
import QtQuick
import Quickshell
import Quickshell.Services.Mpris

Singleton {
    id: root

    readonly property var players: Mpris.players.values
    readonly property var active: {
        const playing = players.find(p => p.isPlaying);
        return playing ?? players.find(p => p.canControl) ?? null;
    }

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

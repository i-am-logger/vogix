// A looped, always-muted video background (curated CC0/PD loops from the
// backgrounds set). Loaded on demand — the QtMultimedia import only
// resolves when the unit's import path carries it, and a missing module
// fails the Loader (logged), never the shell.
import QtQuick
import QtMultimedia
import qs.Services

Video {
    readonly property bool live: (Backgrounds.current?.kind ?? "") === "video"

    source: live ? "file://" + Backgrounds.current.path : ""
    fillMode: VideoOutput.PreserveAspectCrop
    loops: MediaPlayer.Infinite
    muted: true
    volume: 0
    autoPlay: true
}

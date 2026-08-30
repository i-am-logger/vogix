pragma Singleton
// Privacy flags from the Pipewire graph: a live AudioInStream node means
// something is CAPTURING the mic (confident — an app holding a capture
// stream, not merely a device existing). The shell's OWN capture streams
// are excluded — the VU peak monitors register as AudioInStream too, and
// a meter must never light its own privacy dot. Those monitor nodes
// publish NO pipewire properties (verified live), so the only usable
// signal is the node name quickshell's binary gives its streams.
// Screencast detection ships conservative (off) until a signal as
// unambiguous exists for it.
import QtQuick
import Quickshell
import Quickshell.Services.Pipewire

Singleton {
    id: root

    readonly property bool micInUse:
        [...Pipewire.nodes.values].some(n => n.type === PwNodeType.AudioInStream
            && !(n.name ?? "").includes("quickshell"))
    readonly property bool screencast: false
}

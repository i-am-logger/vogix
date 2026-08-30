pragma Singleton
// Privacy flags from the Pipewire graph: a live AudioInStream node means
// something is CAPTURING the mic (confident — an app holding a capture
// stream, not merely a device existing). The shell's OWN audio taps are
// excluded — the VU peak monitors and the cava spectrum subprocess both
// register as AudioInStream, and a meter must never light its own
// privacy dot. The monitor nodes publish NO pipewire properties
// (verified live), so node names are the usable signal; cava is excluded
// by name in general, correctly — it visualizes the OUTPUT monitor, it
// does not record the microphone. Screencast detection ships
// conservative (off) until a signal as unambiguous exists for it.
import QtQuick
import Quickshell
import Quickshell.Services.Pipewire

Singleton {
    id: root

    readonly property bool micInUse:
        [...Pipewire.nodes.values].some(n => {
            const name = n.name ?? "";
            return n.type === PwNodeType.AudioInStream
                && !name.includes("quickshell")
                && name !== "cava";
        })
    readonly property bool screencast: false
}

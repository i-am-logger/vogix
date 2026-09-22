pragma Singleton
// Privacy flags. Microphone, from the Pipewire graph: a live
// AudioInStream node means something is CAPTURING the mic (confident — an
// app holding a capture stream, not merely a device existing). The shell's
// OWN audio taps are excluded — the VU peak monitors and the cava spectrum
// subprocess both register as AudioInStream, and a meter must never light
// its own privacy dot. The monitor nodes publish NO pipewire properties
// (verified live), so node names are the usable signal; cava is excluded
// by name in general, correctly — it visualizes the OUTPUT monitor, it
// does not record the microphone.
//
// Screen, from Hyprland: its screenshare manager posts
// `screencast>>1,<kind>` when a capture session starts delivering frames
// and `>>0,<kind>` when it stops, once each per session, including a
// session torn down mid-cast. Screenshots go through the same manager, so
// a grab shows the glyph for the moment it takes. A cast already running
// when the shell starts is unseen until it ends (Hyprland has no query for
// live sessions).
import QtQuick
import Quickshell
import Quickshell.Hyprland
import Quickshell.Services.Pipewire
import "lib/screencast.js" as Screencast

Singleton {
    id: root

    readonly property bool micInUse:
        [...Pipewire.nodes.values].some(n => {
            const name = n.name ?? "";
            return n.type === PwNodeType.AudioInStream
                && !name.includes("quickshell")
                && name !== "cava"
                && name !== "vogix-scope";
        })

    // Live screen-capture sessions.
    property int screencasts: 0
    readonly property bool screencast: root.screencasts > 0

    Connections {
        target: Hyprland

        function onRawEvent(event: HyprlandEvent): void {
            if (event.name === "screencast")
                root.screencasts = Screencast.step(root.screencasts, event.data);
        }
    }
}

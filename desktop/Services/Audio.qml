pragma Singleton
// Watches the default Pipewire sink and flashes the volume OSD on change.
// The initial binding fire is swallowed — startup must not flash.
import QtQuick
import Quickshell
import Quickshell.Services.Pipewire
import qs.Vogix

Singleton {
    id: root

    readonly property var sink: Pipewire.defaultAudioSink
    property bool primed: false

    PwObjectTracker {
        objects: root.sink ? [root.sink] : []
    }

    Connections {
        target: root.sink?.audio ?? null

        function onVolumesChanged() {
            if (!root.primed) {
                root.primed = true;
                return;
            }
            Osd.show("volume", root.sink.audio.volume, root.sink.audio.muted, "");
        }

        function onMutedChanged() {
            if (root.primed)
                Osd.show("volume", root.sink.audio.volume, root.sink.audio.muted, "");
        }
    }
}

// Microphone mute state; click toggles.
import QtQuick
import Quickshell.Services.Pipewire
import qs.Bar.widgets
import qs.Services
import qs.Vogix

BarText {
    id: root

    readonly property var source: Pipewire.defaultAudioSource

    PwObjectTracker {
        objects: root.source ? [root.source] : []
    }

    text: (root.source?.audio?.muted ?? true) ? "󰍭" : "󰍬"
    color: (root.source?.audio?.muted ?? true)
        ? Tokens.color("bar", "muted")
        : Tokens.color("bar", "foreground")

    MouseArea {
        anchors.fill: parent
        onClicked: {
            if (root.source?.audio)
                root.source.audio.muted = !root.source.audio.muted;
        }
    }
}

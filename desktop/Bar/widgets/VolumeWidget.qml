// Sink volume: icon + percent; click opens the audio panel, middle-click
// mutes, wheel nudges.
import QtQuick
import Quickshell.Services.Pipewire
import qs.Bar.widgets
import qs.Services

BarText {
    id: root

    readonly property var audio: Audio.sink?.audio ?? null

    text: {
        if (!root.audio)
            return "󰖁";
        if (root.audio.muted)
            return "󰝟";
        const pct = Math.round(root.audio.volume * 100);
        return (pct < 34 ? "󰕿" : pct < 67 ? "󰖀" : "󰕾") + " " + pct + "%";
    }

    MouseArea {
        anchors.fill: parent
        acceptedButtons: Qt.LeftButton | Qt.MiddleButton
        onClicked: mouse => {
            if (mouse.button === Qt.MiddleButton && root.audio)
                root.audio.muted = !root.audio.muted;
            else
                Panels.toggle("audio");
        }
        onWheel: wheel => {
            if (root.audio)
                root.audio.volume = Math.max(0, Math.min(1,
                    root.audio.volume + (wheel.angleDelta.y > 0 ? 0.05 : -0.05)));
        }
    }
}

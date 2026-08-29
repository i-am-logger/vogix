// Output volume/mute + input mute.
import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import Quickshell.Services.Pipewire
import qs.Panels
import qs.Services
import qs.Vogix

ColumnLayout {
    id: root

    readonly property var sink: Audio.sink?.audio ?? null
    readonly property var source: Pipewire.defaultAudioSource?.audio ?? null

    spacing: 10

    PanelLabel {
        text: "Audio"
        font.bold: true
        color: Tokens.color("popup", "accent")
    }

    RowLayout {
        Layout.fillWidth: true
        spacing: 10

        PanelLabel {
            text: (root.sink?.muted ?? true) ? "󰝟" : "󰕾"

            MouseArea {
                anchors.fill: parent
                onClicked: {
                    if (root.sink)
                        root.sink.muted = !root.sink.muted;
                }
            }
        }

        Slider {
            Layout.fillWidth: true
            from: 0
            to: 1
            value: root.sink?.volume ?? 0
            onMoved: {
                if (root.sink)
                    root.sink.volume = value;
            }
        }

        PanelLabel {
            text: Math.round((root.sink?.volume ?? 0) * 100) + "%"
        }
    }

    RowLayout {
        Layout.fillWidth: true
        spacing: 10

        PanelLabel {
            text: (root.source?.muted ?? true) ? "󰍭" : "󰍬"
        }

        PanelLabel {
            Layout.fillWidth: true
            text: (root.source?.muted ?? true) ? "Microphone muted" : "Microphone live"
            color: Tokens.color("popup", "muted")
        }

        PanelLabel {
            text: "toggle"
            color: Tokens.color("popup", "accent")

            MouseArea {
                anchors.fill: parent
                onClicked: {
                    if (root.source)
                        root.source.muted = !root.source.muted;
                }
            }
        }
    }
}

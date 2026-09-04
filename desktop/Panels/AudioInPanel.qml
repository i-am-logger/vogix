pragma ComponentBehavior: Bound
// The input-device list: every source pipewire will accept as a
// default, the live one marked, with the mic's mute at the top — the
// two questions a microphone raises ("which one" and "is it open") in
// one place.
import QtQuick
import QtQuick.Layouts
import qs.Panels
import qs.Services
import qs.Vogix

ColumnLayout {
    id: root

    readonly property var devices: Audio.sources

    spacing: 10

    PanelLabel {
        Layout.fillWidth: true
        text: "Input device"
        font.bold: true
        color: Tokens.color("popup", "accent")
    }

    RowLayout {
        Layout.fillWidth: true
        spacing: 10

        PanelLabel {
            text: Audio.micMuted ? "󰍭" : "󰍬"
        }

        PanelLabel {
            Layout.fillWidth: true
            text: Audio.micMuted ? "Microphone muted"
                : (Privacy.micInUse ? "Microphone capturing" : "Microphone live")
            color: Privacy.micInUse && !Audio.micMuted
                ? Tokens.color("popup", "accent")
                : Tokens.color("popup", "muted")
        }

        PanelLabel {
            text: Audio.micMuted ? "unmute" : "mute"
            color: Tokens.color("popup", "accent")

            MouseArea {
                anchors.fill: parent
                onClicked: Audio.toggleMic()
            }
        }
    }

    PanelLabel {
        Layout.fillWidth: true
        visible: root.devices.length === 0
        text: "No input devices"
        color: Tokens.color("popup", "muted")
    }

    ListView {
        Layout.fillWidth: true
        // Grows to the roster, then scrolls — the mute row above must
        // stay reachable however many sources are attached.
        Layout.preferredHeight: Math.min(contentHeight, Metrics.body * 12)
        clip: true
        spacing: 4
        model: root.devices

        delegate: RowLayout {
            id: row

            required property var modelData
            readonly property bool current: Audio.source === row.modelData

            width: ListView.view.width

            PanelLabel {
                Layout.fillWidth: true
                text: (row.current ? "󰸞 " : "󰍬 ") + Audio.fullLabel(row.modelData)
                color: row.current
                    ? Tokens.color("popup", "foreground")
                    : Tokens.color("popup", "muted")
            }

            PanelLabel {
                text: row.current ? "current" : "use"
                color: row.current
                    ? Tokens.color("popup", "muted")
                    : Tokens.color("popup", "accent")

                MouseArea {
                    anchors.fill: parent
                    enabled: !row.current
                    onClicked: Audio.setSource(row.modelData)
                }
            }
        }
    }
}

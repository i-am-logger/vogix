pragma ComponentBehavior: Bound
// The output-device list: every sink pipewire will accept as a default,
// the live one marked. Volume and mute stay in the Audio panel — this
// one answers "which speaker", with room for the full device name the
// rail has to elide.
import QtQuick
import QtQuick.Layouts
import qs.Panels
import qs.Services
import qs.Vogix

ColumnLayout {
    id: root

    readonly property var devices: Audio.sinks

    spacing: 10

    PanelLabel {
        Layout.fillWidth: true
        text: "Output device"
        font.bold: true
        color: Tokens.color("popup", "accent")
    }

    PanelLabel {
        Layout.fillWidth: true
        visible: root.devices.length === 0
        text: "No output devices"
        color: Tokens.color("popup", "muted")
    }

    ListView {
        Layout.fillWidth: true
        // Grows to the roster, then scrolls — a machine with a dozen
        // sinks must not push the switch rows out of the popup.
        Layout.preferredHeight: Math.min(contentHeight, Metrics.body * 12)
        clip: true
        spacing: 4
        model: root.devices

        delegate: RowLayout {
            id: row

            required property var modelData
            readonly property bool current: Audio.sink === row.modelData

            width: ListView.view.width

            PanelLabel {
                Layout.fillWidth: true
                text: (row.current ? "󰸞 " : "󰕾 ") + Audio.fullLabel(row.modelData)
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
                    onClicked: Audio.setSink(row.modelData)
                }
            }
        }
    }
}

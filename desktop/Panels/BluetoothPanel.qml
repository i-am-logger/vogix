pragma ComponentBehavior: Bound
// Bluetooth adapter switch + device list; click connects/disconnects.
import QtQuick
import QtQuick.Layouts
import Quickshell.Bluetooth
import qs.Panels
import qs.Services
import qs.Vogix

ColumnLayout {
    id: root

    readonly property var adapter: Bluetooth.defaultAdapter

    Component.onCompleted: {
        if (root.adapter)
            root.adapter.discovering = true;
    }
    Component.onDestruction: {
        if (root.adapter)
            root.adapter.discovering = false;
    }

    spacing: 10

    RowLayout {
        Layout.fillWidth: true

        PanelLabel {
            Layout.fillWidth: true
            text: "Bluetooth"
            font.bold: true
            color: Tokens.color("popup", "accent")
        }

        PanelLabel {
            text: (root.adapter?.enabled ?? false) ? "on" : "off"
            color: Tokens.color("popup", "accent")

            MouseArea {
                anchors.fill: parent
                onClicked: {
                    if (root.adapter)
                        root.adapter.enabled = !root.adapter.enabled;
                }
            }
        }
    }

    ListView {
        Layout.fillWidth: true
        Layout.fillHeight: true
        Layout.minimumHeight: 200
        clip: true
        spacing: 2
        model: Bluetooth.devices

        delegate: RowLayout {
            id: row

            required property var modelData

            width: ListView.view.width

            PanelLabel {
                Layout.fillWidth: true
                text: (row.modelData.connected ? "󰂱 " : "󰂯 ")
                    + (row.modelData.name || row.modelData.deviceName || row.modelData.address)
            }

            PanelLabel {
                text: row.modelData.connected ? "disconnect"
                    : (row.modelData.paired ? "connect" : "pair")
                color: Tokens.color("popup", "accent")

                MouseArea {
                    anchors.fill: parent
                    onClicked: {
                        if (row.modelData.connected)
                            row.modelData.disconnect();
                        else if (row.modelData.paired)
                            row.modelData.connect();
                        else
                            row.modelData.pair();
                    }
                }
            }
        }
    }
}

pragma ComponentBehavior: Bound
// Wi-Fi scan/connect + wired state, on Quickshell.Networking
// (NetworkManager). A known network connects on click; a new secured one
// opens an inline passphrase field.
import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import Quickshell.Networking
import qs.Panels
import qs.Services
import qs.Vogix

ColumnLayout {
    id: root

    readonly property var devices: Networking.devices.values
    readonly property var wifiDev: devices.find(d => d.type === DeviceType.Wifi) ?? null
    property var pskTarget: null

    // Scan while the panel is open.
    Component.onCompleted: {
        if (root.wifiDev)
            root.wifiDev.scannerEnabled = true;
    }
    Component.onDestruction: {
        if (root.wifiDev)
            root.wifiDev.scannerEnabled = false;
    }

    spacing: 10

    RowLayout {
        Layout.fillWidth: true

        PanelLabel {
            Layout.fillWidth: true
            text: "Network"
            font.bold: true
            color: Tokens.color("popup", "accent")
        }

        PanelLabel {
            visible: NetworkBackend.attached
            text: (Networking.wifiEnabled ?? false) ? "wifi on" : "wifi off"
            color: Tokens.color("popup", "accent")

            MouseArea {
                anchors.fill: parent
                onClicked: Networking.wifiEnabled = !Networking.wifiEnabled
            }
        }
    }

    // No backend means no device or network list at all; say why instead
    // of showing an empty panel.
    PanelLabel {
        visible: !NetworkBackend.attached
        Layout.fillWidth: true
        wrapMode: Text.WordWrap
        text: NetworkBackend.selfRestarts
            ? "NetworkManager is not reachable. The shell restarts itself when it appears."
            : "NetworkManager is not reachable. Restart the shell once it is running."
        color: Tokens.color("popup", "muted")
    }

    Repeater {
        model: root.devices.filter(d => d.type !== DeviceType.Wifi)

        RowLayout {
            id: wiredRow

            required property var modelData

            Layout.fillWidth: true

            PanelLabel {
                text: "󰈀 " + wiredRow.modelData.name
                Layout.fillWidth: true
            }

            PanelLabel {
                text: wiredRow.modelData.connected ? "connected" : "down"
                color: Tokens.color("popup", "muted")
            }
        }
    }

    ListView {
        Layout.fillWidth: true
        Layout.fillHeight: true
        Layout.minimumHeight: 220
        clip: true
        spacing: 2
        model: root.wifiDev ? root.wifiDev.networks : null

        delegate: ColumnLayout {
            id: row

            required property var modelData

            width: ListView.view.width
            spacing: 4

            RowLayout {
                Layout.fillWidth: true

                PanelLabel {
                    Layout.fillWidth: true
                    text: (row.modelData.connected ? "󰸞 " : "󰤨 ") + row.modelData.name
                }

                PanelLabel {
                    text: Math.round((row.modelData.signalStrength ?? 0) * 100) + "%"
                    color: Tokens.color("popup", "muted")
                }

                TapHandler {
                    onTapped: {
                        const net = row.modelData;
                        if (net.connected) {
                            net.disconnect();
                        } else if (net.known || net.security === WifiSecurityType.Open) {
                            net.connect();
                            root.pskTarget = null;
                        } else {
                            root.pskTarget = net;
                        }
                    }
                }
            }

            TextField {
                visible: root.pskTarget === row.modelData
                Layout.fillWidth: true
                echoMode: TextInput.Password
                placeholderText: "Passphrase for " + row.modelData.name
                color: Tokens.color("popup", "foreground")
                font.family: Config.fontFamily
                font.pixelSize: Metrics.body
                background: Rectangle {
                    radius: 4
                    color: Tokens.color("popup", "muted")
                    opacity: 0.2
                }
                onAccepted: {
                    row.modelData.connectWithPsk(text);
                    root.pskTarget = null;
                }
            }
        }
    }
}

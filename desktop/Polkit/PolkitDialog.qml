// The polkit authentication agent + its dialog: a centered prompt with
// exclusive keyboard focus while an authentication flow is active.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Quickshell
import Quickshell.Wayland
import Quickshell.Services.Polkit
import qs.Vogix

Scope {
    id: root

    PolkitAgent {
        id: agent
    }

    PanelWindow {
        visible: agent.flow !== null && !(agent.flow?.isCompleted ?? true)
        anchors {}
        implicitWidth: 440
        implicitHeight: box.implicitHeight + 32
        color: "transparent"
        exclusionMode: ExclusionMode.Ignore
        WlrLayershell.keyboardFocus: visible
            ? WlrKeyboardFocus.Exclusive
            : WlrKeyboardFocus.None

        onVisibleChanged: {
            if (visible) {
                input.text = "";
                input.forceActiveFocus();
            }
        }

        Rectangle {
            id: box
            anchors.centerIn: parent
            width: parent.width - 32
            implicitHeight: col.implicitHeight + 40
            radius: 10
            color: Tokens.color("polkit", "background")
            border.width: 1
            border.color: Tokens.color("polkit", "border")

            ColumnLayout {
                id: col
                anchors {
                    fill: parent
                    margins: 20
                }
                spacing: 10

                Text {
                    Layout.fillWidth: true
                    text: "Authentication required"
                    color: Tokens.color("polkit", "accent")
                    font.family: Config.fontFamily
                    font.pixelSize: Metrics.title
                    font.bold: true
                }

                Text {
                    Layout.fillWidth: true
                    text: agent.flow?.message ?? ""
                    color: Tokens.color("polkit", "foreground")
                    font.family: Config.fontFamily
                    font.pixelSize: Metrics.body
                    wrapMode: Text.Wrap
                }

                Text {
                    Layout.fillWidth: true
                    visible: (agent.flow?.supplementaryMessage ?? "") !== ""
                    text: agent.flow?.supplementaryMessage ?? ""
                    color: (agent.flow?.supplementaryIsError ?? false)
                        ? Tokens.color("polkit", "danger")
                        : Tokens.color("polkit", "muted")
                    font.family: Config.fontFamily
                    font.pixelSize: Metrics.bodySmall
                    wrapMode: Text.Wrap
                }

                TextField {
                    id: input
                    Layout.fillWidth: true
                    visible: agent.flow?.isResponseRequired ?? false
                    echoMode: (agent.flow?.responseVisible ?? false)
                        ? TextInput.Normal
                        : TextInput.Password
                    placeholderText: agent.flow?.inputPrompt ?? "Password"
                    color: Tokens.color("polkit", "foreground")
                    font.family: Config.fontFamily
                    font.pixelSize: Metrics.body
                    onAccepted: agent.flow?.submit(text)
                }

                RowLayout {
                    Layout.alignment: Qt.AlignRight
                    spacing: 12

                    Text {
                        text: "Cancel (Esc)"
                        color: Tokens.color("polkit", "muted")
                        font.family: Config.fontFamily
                        font.pixelSize: Metrics.bodySmall

                        MouseArea {
                            anchors.fill: parent
                            onClicked: agent.flow?.cancelAuthenticationRequest()
                        }
                    }

                    Text {
                        text: "Authenticate (Enter)"
                        color: Tokens.color("polkit", "accent")
                        font.family: Config.fontFamily
                        font.pixelSize: Metrics.bodySmall

                        MouseArea {
                            anchors.fill: parent
                            onClicked: agent.flow?.submit(input.text)
                        }
                    }
                }
            }

            Keys.onEscapePressed: agent.flow?.cancelAuthenticationRequest()
        }
    }
}

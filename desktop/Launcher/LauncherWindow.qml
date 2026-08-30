pragma ComponentBehavior: Bound
// The launcher overlay: one centered panel with a query field and a result
// list, rendering whatever provider qs.Services.Launcher has active.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Quickshell
import Quickshell.Wayland
import qs.Services
import qs.Vogix

PanelWindow {
    id: root

    visible: Launcher.open
    anchors {}
    implicitWidth: 640
    implicitHeight: 460
    color: "transparent"
    exclusionMode: ExclusionMode.Ignore
    WlrLayershell.layer: WlrLayer.Overlay
    WlrLayershell.keyboardFocus: visible
        ? WlrKeyboardFocus.Exclusive
        : WlrKeyboardFocus.None

    onVisibleChanged: {
        if (visible) {
            input.text = Launcher.query;
            input.forceActiveFocus();
        }
    }

    Rectangle {
        anchors.fill: parent
        radius: 10
        color: Tokens.color("launcher", "background")
        border.width: 1
        border.color: Tokens.color("launcher", "border")

        ColumnLayout {
            anchors {
                fill: parent
                margins: 14
            }
            spacing: 10

            RowLayout {
                Layout.fillWidth: true
                spacing: 8

                Text {
                    text: Launcher.prompt !== "" ? Launcher.prompt : Launcher.mode
                    color: Tokens.color("launcher", "accent")
                    font.family: Config.fontFamily
                    font.pixelSize: Metrics.body
                    font.bold: true
                }

                TextField {
                    id: input
                    Layout.fillWidth: true
                    placeholderText: Launcher.mode === "input" ? "Type and press Enter" : "Search…"
                    color: Tokens.color("launcher", "foreground")
                    placeholderTextColor: Tokens.color("launcher", "muted")
                    font.family: Config.fontFamily
                    font.pixelSize: Metrics.body
                    background: Rectangle {
                        radius: 6
                        color: Tokens.color("launcher", "selection")
                    }
                    onTextEdited: Launcher.setQuery(text)
                    onAccepted: Launcher.activate(Launcher.cursor)
                    Keys.onDownPressed: Launcher.moveCursor(1)
                    Keys.onUpPressed: Launcher.moveCursor(-1)
                    Keys.onEscapePressed: Launcher.close()
                }
            }

            ListView {
                id: list
                Layout.fillWidth: true
                Layout.fillHeight: true
                clip: true
                model: Launcher.items
                currentIndex: Launcher.cursor
                onCurrentIndexChanged: positionViewAtIndex(currentIndex, ListView.Contain)

                delegate: Rectangle {
                    id: row

                    required property var modelData
                    required property int index

                    width: list.width
                    height: 36
                    radius: 6
                    color: row.index === Launcher.cursor
                        ? Tokens.color("launcher", "selection")
                        : "transparent"

                    RowLayout {
                        anchors {
                            fill: parent
                            leftMargin: 10
                            rightMargin: 10
                        }
                        spacing: 10

                        Text {
                            visible: row.modelData.icon !== ""
                            text: row.modelData.icon
                            color: Tokens.color("launcher", "accent")
                            font.family: Config.fontFamily
                            font.pixelSize: Metrics.title
                        }

                        Text {
                            Layout.fillWidth: true
                            text: row.modelData.label
                            elide: Text.ElideRight
                            color: Tokens.color("launcher", "foreground")
                            font.family: Config.fontFamily
                            font.pixelSize: Metrics.body
                        }

                        Text {
                            visible: row.modelData.sublabel !== ""
                            text: row.modelData.sublabel
                            color: Tokens.color("launcher", "muted")
                            font.family: Config.fontFamily
                            font.pixelSize: Metrics.bodySmall
                        }
                    }

                    MouseArea {
                        anchors.fill: parent
                        onClicked: Launcher.activate(row.index)
                    }
                }
            }
        }
    }
}

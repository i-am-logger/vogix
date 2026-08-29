// The power menu: a small centered overlay listing the session-ending
// actions from qs.Services.Power. Keyboard-first; Escape closes.
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Wayland
import qs.Services
import qs.Vogix

PanelWindow {
    id: root

    visible: Power.open
    anchors {}
    implicitWidth: 300
    implicitHeight: box.implicitHeight + 24
    color: "transparent"
    exclusionMode: ExclusionMode.Ignore
    WlrLayershell.layer: WlrLayer.Overlay
    WlrLayershell.keyboardFocus: visible
        ? WlrKeyboardFocus.Exclusive
        : WlrKeyboardFocus.None

    onVisibleChanged: {
        if (visible)
            keys.forceActiveFocus();
    }

    Rectangle {
        id: box
        anchors.centerIn: parent
        width: parent.width - 24
        implicitHeight: col.implicitHeight + 28
        radius: 10
        color: Tokens.color("power", "background")
        border.width: 1
        border.color: Tokens.color("power", "border")

        focus: true

        Item {
            id: keys
            focus: true
            Keys.onDownPressed: Power.moveCursor(1)
            Keys.onUpPressed: Power.moveCursor(-1)
            Keys.onEscapePressed: Power.close()
            Keys.onReturnPressed: Power.run(Power.actions[Power.cursor].id)
            Keys.onEnterPressed: Power.run(Power.actions[Power.cursor].id)
        }

        ColumnLayout {
            id: col
            anchors {
                fill: parent
                margins: 14
            }
            spacing: 4

            Repeater {
                model: Power.actions

                delegate: Rectangle {
                    id: row

                    required property var modelData
                    required property int index

                    Layout.fillWidth: true
                    height: 40
                    radius: 6
                    color: row.index === Power.cursor
                        ? Tokens.color("power", "selection")
                        : "transparent"

                    RowLayout {
                        anchors {
                            fill: parent
                            leftMargin: 12
                            rightMargin: 12
                        }
                        spacing: 12

                        Text {
                            text: row.modelData.icon
                            color: row.modelData.id === "poweroff" || row.modelData.id === "reboot"
                                ? Tokens.color("power", "danger")
                                : Tokens.color("power", "accent")
                            font.family: Config.fontFamily
                            font.pixelSize: Config.fontSize + 3
                        }

                        Text {
                            Layout.fillWidth: true
                            text: row.modelData.label
                            color: Tokens.color("power", "foreground")
                            font.family: Config.fontFamily
                            font.pixelSize: Config.fontSize
                        }
                    }

                    MouseArea {
                        anchors.fill: parent
                        onClicked: Power.run(row.modelData.id)
                    }
                }
            }
        }
    }
}

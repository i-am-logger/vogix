// The per-screen lock surface: theme background, a clock, and one password
// box wired to the singular auth flow in qs.Services.Lock. Wrong password
// shakes and stays locked; there is deliberately no cancel.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Quickshell
import Quickshell.Wayland
import qs.Services
import qs.Vogix

WlSessionLockSurface {
    id: surf

    color: Tokens.color("lock", "background")

    SystemClock {
        id: clock
        precision: SystemClock.Minutes
    }

    ColumnLayout {
        anchors.centerIn: parent
        spacing: 24

        Text {
            Layout.alignment: Qt.AlignHCenter
            text: Qt.formatDateTime(clock.date, "HH:mm")
            color: Tokens.color("lock", "foreground")
            font.family: Config.fontFamily
            font.pixelSize: 64
        }

        Text {
            Layout.alignment: Qt.AlignHCenter
            text: Qt.formatDateTime(clock.date, "dddd d MMMM")
            color: Tokens.color("lock", "muted")
            font.family: Config.fontFamily
            font.pixelSize: Metrics.title
        }

        Rectangle {
            id: box

            Layout.alignment: Qt.AlignHCenter
            implicitWidth: 320
            implicitHeight: 48
            radius: 10
            color: Tokens.color("lock", "surface")
            border.width: 1
            border.color: Lock.authError
                ? Tokens.color("lock", "danger")
                : Tokens.color("lock", "accent")

            SequentialAnimation {
                id: shake
                NumberAnimation { target: box; property: "x"; to: box.x - 12; duration: 40 }
                NumberAnimation { target: box; property: "x"; to: box.x + 12; duration: 80 }
                NumberAnimation { target: box; property: "x"; to: box.x; duration: 40 }
            }

            TextField {
                id: input
                anchors.fill: parent
                anchors.margins: 8
                echoMode: TextInput.Password
                placeholderText: Lock.inProgress ? "Authenticating…" : "Password"
                enabled: !Lock.inProgress
                focus: true
                color: Tokens.color("lock", "foreground")
                font.family: Config.fontFamily
                font.pixelSize: Metrics.subtitle
                background: null
                horizontalAlignment: TextInput.AlignHCenter
                onAccepted: {
                    if (text !== "") {
                        Lock.submit(text);
                        text = "";
                    }
                }
            }

            Connections {
                target: Lock

                function onAuthErrorChanged() {
                    if (Lock.authError) {
                        shake.restart();
                        input.forceActiveFocus();
                    }
                }
            }
        }

        Text {
            Layout.alignment: Qt.AlignHCenter
            visible: Lock.authMessage !== ""
            text: Lock.authMessage
            color: Lock.authError
                ? Tokens.color("lock", "danger")
                : Tokens.color("lock", "muted")
            font.family: Config.fontFamily
            font.pixelSize: Metrics.body
        }
    }

    Component.onCompleted: input.forceActiveFocus()
}

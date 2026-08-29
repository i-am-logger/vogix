pragma ComponentBehavior: Bound
// The screensaver stage: a full-screen palette drift (theme colors only)
// with a wandering clock, one per screen, sitting above the session but
// below the lock. Any input resets seat idle, which closes it — the
// overlay itself handles nothing.
import QtQuick
import Quickshell
import Quickshell.Wayland
import qs.Services
import qs.Vogix

Scope {
    Variants {
        model: Quickshell.screens

        PanelWindow {
            id: saver

            required property var modelData

            screen: modelData
            visible: Idle.screensaverActive
            anchors {
                left: true
                right: true
                top: true
                bottom: true
            }
            color: "transparent"
            exclusionMode: ExclusionMode.Ignore
            WlrLayershell.layer: WlrLayer.Overlay

            Rectangle {
                anchors.fill: parent
                color: Theme.semantic.background ?? "black"

                Rectangle {
                    id: blob
                    width: parent.width * 0.5
                    height: width
                    radius: width / 2
                    opacity: 0.12
                    color: Theme.semantic.active ?? "#444444"

                    SequentialAnimation on x {
                        loops: Animation.Infinite
                        running: saver.visible
                        NumberAnimation { to: saver.width * 0.5; duration: 60000; easing.type: Easing.InOutSine }
                        NumberAnimation { to: 0; duration: 60000; easing.type: Easing.InOutSine }
                    }
                    SequentialAnimation on y {
                        loops: Animation.Infinite
                        running: saver.visible
                        NumberAnimation { to: saver.height * 0.5; duration: 83000; easing.type: Easing.InOutSine }
                        NumberAnimation { to: 0; duration: 83000; easing.type: Easing.InOutSine }
                    }
                }

                Text {
                    id: clockText

                    SystemClock {
                        id: saverClock
                        precision: SystemClock.Minutes
                    }

                    text: Qt.formatDateTime(saverClock.date, "HH:mm")
                    color: Theme.semantic.foreground_comment ?? "#888888"
                    font.family: Config.fontFamily
                    font.pixelSize: 64

                    SequentialAnimation on x {
                        loops: Animation.Infinite
                        running: saver.visible
                        NumberAnimation { to: saver.width - clockText.width - 80; duration: 127000; easing.type: Easing.InOutSine }
                        NumberAnimation { to: 80; duration: 127000; easing.type: Easing.InOutSine }
                    }
                    SequentialAnimation on y {
                        loops: Animation.Infinite
                        running: saver.visible
                        NumberAnimation { to: saver.height - clockText.height - 80; duration: 101000; easing.type: Easing.InOutSine }
                        NumberAnimation { to: 80; duration: 101000; easing.type: Easing.InOutSine }
                    }
                }
            }
        }
    }
}

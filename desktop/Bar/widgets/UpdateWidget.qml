// System-update state, the NixOS way: visible only while the booted system
// generation differs from the current one — an update was applied and a
// reboot finishes it. No polling a package index; the store links ARE the
// state. Click shows the two generations as a notification.
import QtQuick
import Quickshell
import Quickshell.Io
import qs.Bar.widgets
import qs.Vogix

BarText {
    id: root

    property bool rebootPending: false

    visible: rebootPending
    text: "󰚰"
    color: Tokens.color("bar", "accent")

    Timer {
        interval: 5 * 60 * 1000
        running: true
        repeat: true
        triggeredOnStart: true
        onTriggered: {
            proc.running = false;
            proc.running = true;
        }
    }

    Process {
        id: proc
        command: ["readlink", "/run/booted-system", "/run/current-system"]
        stdout: StdioCollector {
            onStreamFinished: {
                const lines = text.trim().split("\n");
                root.rebootPending = lines.length === 2 && lines[0] !== lines[1];
            }
        }
    }

    MouseArea {
        anchors.fill: parent
        onClicked: Quickshell.execDetached(["sh", "-c",
            "notify-send 'Reboot pending' \"booted: $(readlink /run/booted-system)\nCURRENT: $(readlink /run/current-system)\""])
    }
}

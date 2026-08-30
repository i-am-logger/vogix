pragma Singleton
// The focused window's identity and geometry, tracked the reliable way:
// `hyprctl -j activewindow` on a debounced event trigger. Quickshell's
// Hyprland.activeToplevel stays null on this compositor (no
// hyprland-toplevel-mapping-v1), so both the window title and the focus
// brackets read from here instead.
import QtQuick
import Quickshell
import Quickshell.Hyprland
import Quickshell.Io

Singleton {
    id: root

    // null when nothing is focused; else { x, y, w, h, fullscreen }.
    property var geometry: null
    property string title: ""
    property string appClass: ""

    Component.onCompleted: refresh.restart()

    Connections {
        target: Hyprland

        function onRawEvent(event: HyprlandEvent): void {
            switch (event.name) {
            case "activewindow":
            case "activewindowv2":
            case "openwindow":
            case "closewindow":
            case "movewindow":
            case "movewindowv2":
            case "workspace":
            case "workspacev2":
            case "fullscreen":
            case "changefloatingmode":
            case "focusedmon":
            case "configreloaded":
                refresh.restart();
                break;
            }
        }
    }

    Timer {
        id: refresh
        interval: 30
        onTriggered: proc.running = true
    }

    Process {
        id: proc
        command: ["hyprctl", "-j", "activewindow"]

        stdout: StdioCollector {
            onStreamFinished: {
                let doc;
                try {
                    doc = JSON.parse(text);
                } catch (e) {
                    doc = {};
                }
                if (doc.at === undefined) {
                    root.geometry = null;
                    root.title = "";
                    root.appClass = "";
                    return;
                }
                root.geometry = {
                    x: doc.at[0],
                    y: doc.at[1],
                    w: doc.size[0],
                    h: doc.size[1],
                    fullscreen: doc.fullscreen ?? 0
                };
                root.title = doc.title ?? "";
                root.appClass = doc.class ?? "";
            }
        }
    }
}

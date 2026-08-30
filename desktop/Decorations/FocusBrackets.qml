pragma ComponentBehavior: Bound
// The Flight Deck focus brackets: four accent L-brackets at the corners
// of the FOCUSED window, drawn by a click-through overlay per screen —
// Hyprland borders are full-perimeter only, so this is the shell's job.
// Geometry follows the compositor over IPC (event-driven refresh with a
// short debounce), which means brackets glide to the drop point after a
// drag rather than chasing it. Hidden on real fullscreen.
import QtQuick
import Quickshell
import Quickshell.Wayland
import Quickshell.Hyprland
import qs.Vogix

Scope {
    id: root

    readonly property int arm: 14
    readonly property int thickness: 2
    readonly property int pad: 3

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
            case "configreloaded":
                refreshDebounce.restart();
                break;
            }
        }
    }

    Timer {
        id: refreshDebounce
        interval: 30
        onTriggered: Hyprland.refreshToplevels()
    }

    Variants {
        model: Quickshell.screens

        PanelWindow {
            id: overlay

            required property var modelData

            readonly property var active: Hyprland.activeToplevel
            readonly property var ipc: overlay.active?.lastIpcObject ?? ({})
            readonly property var hyprMonitor: Hyprland.monitorFor(overlay.screen)
            readonly property bool onThisScreen:
                overlay.active !== null
                && overlay.active.monitor !== null
                && overlay.hyprMonitor !== null
                && overlay.active.monitor === overlay.hyprMonitor

            // Window rect in this screen's logical coordinates, padded out
            // so the brackets sit just outside the window edge.
            readonly property var at: overlay.ipc.at ?? [0, 0]
            readonly property var size: overlay.ipc.size ?? [0, 0]

            screen: overlay.modelData
            visible: overlay.onThisScreen
                && overlay.ipc.at !== undefined
                && (overlay.ipc.fullscreen ?? 0) < 2

            anchors {
                top: true
                bottom: true
                left: true
                right: true
            }
            exclusionMode: ExclusionMode.Ignore
            WlrLayershell.layer: WlrLayer.Top
            color: "transparent"

            // Empty input region: the overlay is pure paint, every click
            // falls through to the window beneath.
            mask: Region {}

            component Bracket: Item {
                property bool alignRight: false
                property bool alignBottom: false

                width: root.arm
                height: root.arm

                Rectangle {
                    width: parent.width
                    height: root.thickness
                    y: parent.alignBottom ? parent.height - height : 0
                    color: Tokens.color("bar", "accent")
                }

                Rectangle {
                    width: root.thickness
                    height: parent.height
                    x: parent.alignRight ? parent.width - width : 0
                    color: Tokens.color("bar", "accent")
                }
            }

            Item {
                id: target

                x: overlay.at[0] - (overlay.screen?.x ?? 0) - root.pad
                y: overlay.at[1] - (overlay.screen?.y ?? 0) - root.pad
                width: overlay.size[0] + root.pad * 2
                height: overlay.size[1] + root.pad * 2

                Behavior on x { NumberAnimation { duration: 90; easing.type: Easing.OutCubic } }
                Behavior on y { NumberAnimation { duration: 90; easing.type: Easing.OutCubic } }
                Behavior on width { NumberAnimation { duration: 90; easing.type: Easing.OutCubic } }
                Behavior on height { NumberAnimation { duration: 90; easing.type: Easing.OutCubic } }

                Bracket {}

                Bracket {
                    x: parent.width - width
                    alignRight: true
                }

                Bracket {
                    y: parent.height - height
                    alignBottom: true
                }

                Bracket {
                    x: parent.width - width
                    y: parent.height - height
                    alignRight: true
                    alignBottom: true
                }
            }
        }
    }
}

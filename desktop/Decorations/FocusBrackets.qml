pragma ComponentBehavior: Bound
// The Flight Deck focus brackets: four accent L-brackets at the corners
// of the FOCUSED window, drawn by a click-through overlay per screen —
// Hyprland borders are full-perimeter only, so this is the shell's job.
// Geometry comes from qs.Services.ActiveWindow (event-driven hyprctl);
// brackets glide to a drag's drop point rather than chasing it. Hidden
// on real fullscreen. The WINDOW stays mapped (remapping a layer costs
// ~150 ms and focus changes constantly); only the bracket item toggles.
import QtQuick
import Quickshell
import Quickshell.Wayland
import qs.Services
import qs.Vogix

Scope {
    id: root

    readonly property int arm: 14
    readonly property int thickness: 2
    readonly property int pad: 3

    Variants {
        model: Quickshell.screens

        PanelWindow {
            id: overlay

            required property var modelData

            readonly property var win: ActiveWindow.geometry
            // The window is "on this screen" when its top-left corner is.
            readonly property bool showBrackets: overlay.win !== null
                && overlay.win.fullscreen < 2
                && overlay.win.x >= overlay.screen.x
                && overlay.win.x < overlay.screen.x + overlay.screen.width
                && overlay.win.y >= overlay.screen.y
                && overlay.win.y < overlay.screen.y + overlay.screen.height

            screen: overlay.modelData
            visible: true

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

                visible: overlay.showBrackets
                x: (overlay.win?.x ?? 0) - overlay.screen.x - root.pad
                y: (overlay.win?.y ?? 0) - overlay.screen.y - root.pad
                width: (overlay.win?.w ?? 0) + root.pad * 2
                height: (overlay.win?.h ?? 0) + root.pad * 2

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

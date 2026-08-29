pragma ComponentBehavior: Bound
// The dim stage: a click-through veil on every screen while idle. Any input
// wakes the idle monitor, which drops `dimmed` and the veil with it.
import QtQuick
import Quickshell
import qs.Services
import qs.Vogix

Scope {
    Variants {
        model: Quickshell.screens

        PanelWindow {
            required property var modelData

            screen: modelData
            visible: Idle.dimmed
            exclusionMode: ExclusionMode.Ignore
            anchors {
                top: true
                bottom: true
                left: true
                right: true
            }
            color: "transparent"
            // Empty input region: the veil never eats the wake-up click.
            mask: Region {}

            Rectangle {
                anchors.fill: parent
                color: Qt.alpha(Theme.semantic.background ?? "#000000", 0.6)
            }
        }
    }
}

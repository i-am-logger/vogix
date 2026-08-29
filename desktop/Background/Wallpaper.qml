pragma ComponentBehavior: Bound
// The wallpaper: a background-layer window per screen, crossfading between
// backgrounds so a theme switch dissolves with the palette instead of
// popping. Non-image kinds (shader/video) arrive in a later increment; an
// entry the renderer doesn't know falls back to the theme background color,
// which is always correct.
import QtQuick
import Quickshell
import Quickshell.Wayland
import qs.Services
import qs.Vogix

Scope {
    Variants {
        model: Quickshell.screens

        PanelWindow {
            id: wall

            required property var modelData
            readonly property string sourcePath:
                (Backgrounds.current && Backgrounds.current.kind === "image")
                    || (Backgrounds.current && Backgrounds.current.kind === "generated")
                    ? "file://" + Backgrounds.current.path
                    : ""

            screen: modelData
            visible: (Config.doc.background ?? {}).enable ?? true
            WlrLayershell.layer: WlrLayer.Background
            exclusionMode: ExclusionMode.Ignore
            anchors {
                top: true
                bottom: true
                left: true
                right: true
            }
            color: Theme.semantic.background ?? "#101010"
            mask: Region {}

            // Two slots; the hidden one loads the next image, then fades in.
            Image {
                id: slotA
                anchors.fill: parent
                fillMode: Image.PreserveAspectCrop
                asynchronous: true
                opacity: wall.showingA ? 1 : 0
                Behavior on opacity { NumberAnimation { duration: 400 } }
            }

            Image {
                id: slotB
                anchors.fill: parent
                fillMode: Image.PreserveAspectCrop
                asynchronous: true
                opacity: wall.showingA ? 0 : 1
                Behavior on opacity { NumberAnimation { duration: 400 } }
            }

            property bool showingA: true

            onSourcePathChanged: {
                if (sourcePath === "") {
                    slotA.source = "";
                    slotB.source = "";
                    return;
                }
                if (showingA)
                    slotB.source = sourcePath;
                else
                    slotA.source = sourcePath;
                showingA = !showingA;
            }

            Component.onCompleted: {
                if (sourcePath !== "") {
                    slotA.source = sourcePath;
                    showingA = true;
                }
            }
        }
    }
}

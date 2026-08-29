pragma ComponentBehavior: Bound
// The wallpaper: a background-layer window per screen, crossfading between
// image backgrounds so a theme switch dissolves with the palette instead
// of popping. Live kinds render too: `shader` runs the built-in aurora
// (palette as uniforms, one shader for every theme and polarity) and
// `video` loops muted through QtMultimedia — both obey the animate policy
// (pause on battery, while idle-dimmed, or entirely; desktop.json
// background.animate = always | on-ac | never). An entry the renderer
// doesn't know falls back to the theme background color, always correct.
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
            readonly property string kind: Backgrounds.current?.kind ?? ""
            readonly property string sourcePath:
                wall.kind === "image" || wall.kind === "generated"
                    ? "file://" + Backgrounds.current.path
                    : ""
            // The animate policy: live kinds run always / on mains power /
            // never — and always hold while the idle dim stage is up.
            readonly property string animate:
                (Config.doc.background ?? {}).animate ?? "on-ac"
            readonly property bool liveAllowed:
                wall.animate === "always"
                    || (wall.animate === "on-ac" && !Battery.onBattery)

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

            // The live-background layers, above the image slots (which hold
            // the fallback while a live kind is active).
            Loader {
                anchors.fill: parent
                active: wall.kind === "shader"
                sourceComponent: ShaderLayer {
                    running: wall.visible && wall.liveAllowed && !Idle.dimmed
                }
            }

            Loader {
                anchors.fill: parent
                active: wall.kind === "video"
                source: "VideoLayer.qml"
                onStatusChanged: {
                    if (status === Loader.Error)
                        console.warn("vogix: video backgrounds need QtMultimedia in the import path");
                }
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

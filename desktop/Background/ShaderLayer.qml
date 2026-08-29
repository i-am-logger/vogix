// The built-in "aurora" live background: the precompiled fragment shader
// (data/aurora.frag.qsb, baked at package build) with the CURRENT theme's
// slots as uniforms — a theme switch recolors the sky live. `running`
// implements the pause policy: the clock stops, the last frame stands, the
// GPU goes quiet. Degrades to 15 fps by design — a background, not a game.
import QtQuick
import Quickshell
import qs.Vogix

ShaderEffect {
    id: layer

    property bool running: true

    property real time: 0
    property color base00: Theme.semantic.background ?? "#101010"
    property color base01: Theme.semantic.background_surface ?? "#181818"
    property color accentA: Theme.semantic.active ?? "#5a9aaa"
    property color accentB: Theme.semantic.link ?? "#6a8ac0"

    fragmentShader: "file://" + Quickshell.shellDir + "/data/aurora.frag.qsb"

    FrameAnimation {
        running: layer.running
        onTriggered: {
            // ~15 fps is plenty for a slow drift; skip frames instead of
            // rendering every vsync.
            layer.time += frameTime;
        }
    }
}

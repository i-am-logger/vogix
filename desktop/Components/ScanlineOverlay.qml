// The opt-in CRT texture over chrome surfaces (bars, notification
// cards), gated on background.scanlines. Static shader — zero per-frame
// cost; nobody else in the quickshell ecosystem does phosphor styling.
import QtQuick
import Quickshell
import qs.Vogix

Loader {
    anchors.fill: parent
    active: (Config.doc.background ?? {}).scanlines ?? false

    sourceComponent: ShaderEffect {
        readonly property real pixelHeight: height

        fragmentShader: "file://" + Quickshell.shellDir + "/data/scanline.frag.qsb"
    }
}

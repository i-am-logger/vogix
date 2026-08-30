pragma Singleton
// The shell's type + density scale: every size derives multiplicatively
// from ONE root (desktop.json font.size), so a person changes a single
// number and the whole HUD scales. The multiplier table is a shell design
// constant (the v2 Rust shell reimplements the same table) — deliberately
// NOT in desktop.json.
import QtQuick
import Quickshell

Singleton {
    id: root

    // The rem root.
    readonly property int body: Config.fontSize

    // Type tokens (multiplicative, rounded).
    readonly property int micro: Math.round(body * 0.72)
    readonly property int caption: Math.round(body * 0.833)
    readonly property int bodySmall: Math.round(body * 0.917)
    readonly property int subtitle: Math.round(body * 1.083)
    readonly property int title: Math.round(body * 1.167)
    readonly property int heading: Math.round(body * 1.333)
    readonly property int display: body * 2

    // Density units: spacing = unit * n; chip = the square hit target for
    // workspace boxes / mode pills / tray icons.
    readonly property int unit: Math.max(2, Math.round(body * 0.25))
    readonly property int chip: body + 8
}

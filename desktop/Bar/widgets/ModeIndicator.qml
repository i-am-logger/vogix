// The input engine's current mode (the mode-visibility surface the border
// color already carries; the bar is its second reader). Label AND color
// come from the same modeColors table the engine uses (input.json:
// { slot, label } per mode) — one table, two surfaces. The root mode
// stays muted; any other mode gets its slot's color as the chip.
import QtQuick
import Quickshell.Io
import qs.Bar.widgets
import qs.Vogix

Rectangle {
    id: root

    property var table: ({})

    readonly property var entry: root.table[Mode.mode] ?? null
    readonly property bool isRoot: Mode.mode === "app"

    implicitWidth: label.implicitWidth + Metrics.unit * 3
    implicitHeight: Metrics.chip
    color: {
        if (root.isRoot)
            return "transparent";
        const slot = root.entry?.slot ?? "";
        const hex = Theme.semantic[slot];
        return hex ?? Tokens.color("bar", "accent");
    }

    BarText {
        id: label
        anchors.centerIn: parent
        // Bracketed, uppercase — the [MODE] readout of the HUD.
        text: "[" + (root.entry?.label !== undefined && root.entry.label !== ""
            ? root.entry.label
            : Mode.mode) + "]"
        font.capitalization: Font.AllUppercase
        font.letterSpacing: 1
        color: root.isRoot
            ? Tokens.color("bar", "muted")
            : Tokens.color("bar", "background")
    }

    FileView {
        path: Paths.stateRoot + "/input.json"
        watchChanges: false
        onLoaded: {
            try {
                root.table = JSON.parse(text()).modeColors ?? {};
            } catch (e) {
                root.table = {};
            }
        }
        onLoadFailed: root.table = {}
    }
}

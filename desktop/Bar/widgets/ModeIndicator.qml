// The input engine's current mode (the mode-visibility surface the border
// color already carries; the bar is its second reader). Label AND color
// come from the same modeColors table the engine uses (input.json:
// { slot, label } per mode) — one table, two surfaces. Horizontal: the
// framed MODE cell, letter-spaced bold in the mode's color. Vertical
// rail: a single-letter box whose frame takes the mode color.
import QtQuick
import Quickshell.Io
import qs.Bar.widgets
import qs.Vogix

FrameCell {
    id: root

    property BarAxis axis: null
    readonly property bool vertical: axis?.vertical ?? false

    property var table: ({})

    readonly property var entry: root.table[Mode.mode] ?? null
    readonly property string modeLabel: entry?.label !== undefined && entry.label !== ""
        ? entry.label
        : Mode.mode
    readonly property color modeColor: {
        const slot = root.entry?.slot ?? "";
        const hex = Theme.semantic[slot];
        return hex ?? Tokens.color("bar", "accent");
    }

    title: vertical ? "" : "MODE"
    frameColor: vertical ? root.modeColor : Tokens.color("meter", "frame")
    padH: vertical ? 5 : 10
    padV: 3

    BarText {
        text: root.vertical ? root.modeLabel.slice(0, 1) : root.modeLabel
        font.pixelSize: Metrics.bodySmall
        font.bold: true
        font.letterSpacing: root.vertical ? 0 : 3
        font.capitalization: Font.AllUppercase
        color: root.modeColor
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
        // A failed load must clear the table, not leave the last good one in
        // place: a stale color is indistinguishable from a current one, so the
        // chip would keep asserting a mode mapping that no longer exists.
        onLoadFailed: root.table = {}
    }
}

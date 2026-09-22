// A custom cell (`custom/<name>` in a bar layout): the output of a command
// the user configured in desktop.json `custom`, in the Flight Deck
// stat-cell shape — the break-title frame, an optional gauge, then the
// value. json output sets the gauge and the value's level color; a failed
// run reads ERR in the danger color. A click runs the entry's onClick (or
// re-runs the command when it has none). On a rail the title cuts to four
// characters and the value elides at the bar's width.
import QtQuick
import qs.Bar.widgets
import qs.Components
import qs.Services
import qs.Vogix

FrameCell {
    id: root

    property BarAxis axis: null
    // Injected by Section after load: "custom/<name>".
    property string widgetName: ""

    readonly property bool vertical: axis?.vertical ?? false
    readonly property string cellName: widgetName.startsWith("custom/") ? widgetName.slice(7) : ""
    readonly property var def: (Config.doc.custom ?? {})[cellName] ?? ({})
    readonly property var runner: Custom.runner(cellName)
    readonly property bool failed: (runner?.failure ?? "") !== ""
    readonly property string level: failed ? "danger" : (runner?.level ?? "normal")
    readonly property real meterValue: failed ? -1 : (runner?.meter ?? -1)
    // Room for the value inside a rail: the bar's width less the cell's
    // side padding and a unit of air on each side.
    readonly property real railRoom: (axis?.thickness ?? 0) - 2 * padH - 4 * Metrics.unit

    // The name arrives after the cell is created, so the runner is held
    // from whichever name the cell currently shows.
    property string _held: ""

    function _hold(next: string): void {
        if (root._held !== "")
            Custom.release(root._held);
        root._held = next;
        if (next !== "")
            Custom.acquire(next);
    }

    onCellNameChanged: _hold(cellName)
    Component.onDestruction: _hold("")

    title: {
        const t = root.def.title ?? root.cellName;
        return root.vertical ? t.slice(0, 4) : t;
    }
    padH: vertical ? 6 : 8
    padV: vertical ? 6 : 3
    interactive: true
    onClicked: Custom.click(root.cellName)

    TextMetrics {
        id: reserve
        font: valueText.font
        text: root.def.widest ?? ""
    }

    Grid {
        columns: root.vertical ? 1 : 2
        spacing: root.vertical ? Metrics.unit : Metrics.unit * 2
        verticalItemAlignment: Grid.AlignVCenter
        horizontalItemAlignment: Grid.AlignHCenter

        SegmentedMeter {
            visible: root.meterValue >= 0
            vertical: root.vertical
            width: root.vertical ? Math.round(Metrics.body * 0.75) : Metrics.body * 3
            height: root.vertical ? Metrics.body * 3.5 : Math.round(Metrics.body * 0.55)
            value: Math.max(0, root.meterValue)
            low: Tokens.color("meter", "low")
            mid: Tokens.color("meter", "mid")
            high: Tokens.color("meter", "high")
            unlit: Tokens.color("meter", "unlit")
            capColor: Tokens.color("meter", "cap")
        }

        NumericText {
            id: valueText

            readonly property real natural: Math.ceil(Math.max(implicitWidth, reserve.width))

            text: root.failed ? "ERR" : (root.runner?.text ?? "")
            width: root.vertical && root.railRoom > 0 ? Math.min(natural, root.railRoom) : natural
            elide: Text.ElideRight
            font.pixelSize: Metrics.bodySmall
            color: root.level === "danger" ? Tokens.color("meter", "high")
                : root.level === "warning" ? Tokens.color("meter", "mid")
                : Tokens.color("bar", "foreground")
        }
    }
}

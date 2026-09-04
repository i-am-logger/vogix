pragma ComponentBehavior: Bound
// The capacity row: one break-title cell per gauge meters.mounts names,
// so /nix, /persist and swap read as the same instrument as MEM and CPU
// rather than as a second kind of thing. A gauge this host does not have
// is ABSENT — SysStat omits it, because a 0% cell would claim an empty
// filesystem where there is none at all. Gauge then number, like every
// stat cell. Filesystems keep the 80/95 thresholds a
// filling disk is urgent at on every host; swap keeps its own
// (meters.thresholds.swap), because swap filling means something else.
//
// Horizontal bars are the home (the run of stat cells along the bottom).
// On a vertical bar the cells stack and the titles cut to four
// characters, since the title is what sets a cell's width and a rail is
// narrow.
import QtQuick
import QtQuick.Layouts
import qs.Bar.widgets
import qs.Services
import qs.Vogix

GridLayout {
    id: root

    property BarAxis axis: null

    readonly property bool vertical: axis?.vertical ?? false
    readonly property var swapTh: ((Config.doc.meters ?? {}).thresholds ?? {}).swap ?? ({})

    function fmt(rate: real): string {
        if (rate >= 1024 * 1024)
            return (rate / (1024 * 1024)).toFixed(1) + "M";
        if (rate >= 1024)
            return Math.round(rate / 1024) + "K";
        return Math.round(rate) + "B";
    }

    // The last path segment, uppercased: /nix → NIX, / → ROOT.
    function label(point: string): string {
        if (point === "swap")
            return "SWAP";
        const seg = point.split("/").filter(s => s !== "").pop() ?? "ROOT";
        return (root.vertical ? seg.slice(0, 4) : seg).toUpperCase();
    }

    flow: vertical ? GridLayout.TopToBottom : GridLayout.LeftToRight
    rowSpacing: Metrics.unit * 2
    columnSpacing: Metrics.unit * 2

    Repeater {
        model: SysStat.gaugesPresent

        StatCell {
            id: cell

            required property string modelData

            axis: root.axis

            readonly property bool isSwap: cell.modelData === "swap"
            readonly property real used: SysStat.gaugeUsed(cell.modelData)
            readonly property int pct: Math.round(cell.used * 100)
            readonly property int warnPct: cell.isSwap ? (root.swapTh.warn ?? 20) : 80
            readonly property int dangerPct: cell.isSwap ? (root.swapTh.danger ?? 80) : 95

            // The model is already filtered, so this only catches a mount
            // vanishing between one df and the next repaint.
            visible: cell.used >= 0
            title: root.label(cell.modelData)
            value: String(cell.pct).padStart(3, "0") + "%"
            widestValue: "000%"
            meterValue: Math.max(0, cell.used)
            meterWarnAt: cell.warnPct / 100
            meterDangerAt: cell.dangerPct / 100
            // The filesystem's own live throughput — absent where no
            // block device backs the mount (tmpfs must show nothing).
            subText: (SysStat.gaugeIo[cell.modelData] !== undefined)
                ? "⇅" + root.fmt(SysStat.gaugeIo[cell.modelData])
                : ""
            valueColor: cell.pct >= cell.dangerPct ? Tokens.color("meter", "high")
                : cell.pct >= cell.warnPct ? Tokens.color("meter", "mid")
                : Tokens.color("bar", "foreground")
        }
    }
}

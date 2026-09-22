pragma ComponentBehavior: Bound
// The fan row: one break-title cell per hwmon fan that has spun this
// session (SysStat.fansPresent), its RPM colored by the
// meters.thresholds.fan pair. Gauge then number where the chip reports
// the fan's top speed, number only where it does not. When the fans come
// from more than one chip, each cell names its chip under the reading,
// since FAN1 then means two different headers.
//
// On a vertical bar the cells stack and the titles cut to four
// characters, like the mount cells.
import QtQuick
import QtQuick.Layouts
import qs.Bar.widgets
import qs.Services
import qs.Vogix
import "../../Services/lib/fans.js" as Fans

GridLayout {
    id: root

    property BarAxis axis: null

    readonly property bool vertical: axis?.vertical ?? false
    readonly property var th: ((Config.doc.meters ?? {}).thresholds ?? {}).fan ?? ({})
    readonly property int warnRpm: th.warn ?? 3000
    readonly property int dangerRpm: th.danger ?? 4500
    readonly property bool showChip: Fans.spansChips(SysStat.fans)

    flow: vertical ? GridLayout.TopToBottom : GridLayout.LeftToRight
    rowSpacing: Metrics.unit * 2
    columnSpacing: Metrics.unit * 2

    Repeater {
        model: SysStat.fansPresent

        StatCell {
            id: cell

            required property string modelData

            readonly property var fan: SysStat.fan(cell.modelData)
            readonly property int rpm: SysStat.fanRpm[cell.modelData] ?? 0
            readonly property real maxRpm: cell.fan?.maxRpm ?? 0

            axis: root.axis
            visible: cell.fan !== null
            title: cell.fan ? Fans.title(cell.fan, root.vertical) : ""
            value: String(cell.rpm).padStart(4, "0")
            widestValue: "0000"
            meterValue: cell.fan ? Fans.gauge(cell.fan, cell.rpm) : -1
            meterWarnAt: cell.maxRpm > 0 ? root.warnRpm / cell.maxRpm : 1
            meterDangerAt: cell.maxRpm > 0 ? root.dangerRpm / cell.maxRpm : 1
            // The reserved sub-readout width holds seven characters.
            subText: root.showChip && cell.fan ? cell.fan.chip.slice(0, 7) : ""
            valueColor: cell.rpm >= root.dangerRpm ? Tokens.color("meter", "high")
                : cell.rpm >= root.warnRpm ? Tokens.color("meter", "mid")
                : Tokens.color("bar", "foreground")
        }
    }

    // Held while this bar is on screen: the tachometer reads on the slow
    // tick.
    Lease {
        active: root.axis?.live ?? false
        onAcquire: SysStat.acquire(["fans"])
        onRelease: SysStat.release(["fans"])
    }
}

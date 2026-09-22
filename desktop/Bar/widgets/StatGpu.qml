// GPU busy cell — gauge over number over trace, exactly the CPU cell's
// shape, sitting right below it, colored by the meters.thresholds.gpu
// pair. Absent on hosts where SysStat finds no GPU it can measure
// without privileges.
import QtQuick
import qs.Bar.widgets
import qs.Services
import qs.Vogix

StatCell {
    id: root

    readonly property int pct: Math.round(SysStat.gpuBusy * 100)
    readonly property var th: ((Config.doc.meters ?? {}).thresholds ?? {}).gpu ?? ({})

    visible: SysStat.hasGpu
    title: "GPU"
    value: String(pct).padStart(3, "0") + "%"
    widestValue: "000%"
    meterValue: SysStat.gpuBusy
    meterWarnAt: (th.warn ?? 60) / 100
    meterDangerAt: (th.danger ?? 90) / 100
    traceValues: SysStat.gpuHistory
    valueColor: pct >= (th.danger ?? 90) ? Tokens.color("meter", "high")
        : pct >= (th.warn ?? 60) ? Tokens.color("meter", "mid")
        : Tokens.color("bar", "foreground")

    // Held while this bar is on screen.
    Lease {
        active: root.axis?.live ?? false
        onAcquire: SysStat.acquire(["gpu"])
        onRelease: SysStat.release(["gpu"])
    }
}

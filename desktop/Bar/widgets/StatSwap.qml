// Swap-in-use cell; hidden on hosts with no swap at all. Swap filling is
// an early-warning signal, hence the low thresholds.
import QtQuick
import qs.Bar.widgets
import qs.Services
import qs.Vogix

StatCell {
    id: root

    readonly property int pct: Math.round(SysStat.swap * 100)
    readonly property var th: ((Config.doc.meters ?? {}).thresholds ?? {}).swap ?? ({})

    visible: SysStat.hasSwap
    title: "SWAP"
    value: String(pct).padStart(3, "0") + "%"
    widestValue: "000%"
    meterValue: SysStat.swap
    meterWarnAt: (th.warn ?? 20) / 100
    meterDangerAt: (th.danger ?? 80) / 100
    valueColor: pct >= (th.danger ?? 80) ? Tokens.color("meter", "high")
        : pct >= (th.warn ?? 20) ? Tokens.color("meter", "mid")
        : Tokens.color("bar", "foreground")

    // Held while this bar is on screen.
    Lease {
        active: root.axis?.live ?? false
        onAcquire: SysStat.acquire(["memory"])
        onRelease: SysStat.release(["memory"])
    }
}

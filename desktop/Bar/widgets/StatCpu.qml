// CPU load cell, colored by the meters.thresholds.cpu pair.
import QtQuick
import qs.Bar.widgets
import qs.Services
import qs.Vogix

StatCell {
    readonly property int pct: Math.round(SysStat.cpu * 100)
    readonly property var th: ((Config.doc.meters ?? {}).thresholds ?? {}).cpu ?? ({})

    title: "CPU"
    value: String(pct).padStart(3, "0") + "%"
    widestValue: "000%"
    meterValue: SysStat.cpu
    meterWarnAt: (th.warn ?? 50) / 100
    meterDangerAt: (th.danger ?? 90) / 100
    traceValues: SysStat.cpuHistory
    valueColor: pct >= (th.danger ?? 90) ? Tokens.color("meter", "high")
        : pct >= (th.warn ?? 50) ? Tokens.color("meter", "mid")
        : Tokens.color("bar", "foreground")
}

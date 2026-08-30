// Memory-in-use cell, colored by the meters.thresholds.memory pair.
import QtQuick
import qs.Bar.widgets
import qs.Services
import qs.Vogix

StatCell {
    readonly property int pct: Math.round(SysStat.memory * 100)
    readonly property var th: ((Config.doc.meters ?? {}).thresholds ?? {}).memory ?? ({})

    label: "MEM"
    value: pct + "%"
    widestValue: "100%"
    valueColor: pct >= (th.danger ?? 90) ? Tokens.color("meter", "high")
        : pct >= (th.warn ?? 60) ? Tokens.color("meter", "mid")
        : Tokens.color("bar", "foreground")
}

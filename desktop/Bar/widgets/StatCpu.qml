// CPU load cell, colored by the meters.thresholds.cpu pair.
import QtQuick
import qs.Bar.widgets
import qs.Services
import qs.Vogix

StatCell {
    readonly property int pct: Math.round(SysStat.cpu * 100)
    readonly property var th: ((Config.doc.meters ?? {}).thresholds ?? {}).cpu ?? ({})

    label: "CPU"
    value: pct + "%"
    widestValue: "100%"
    valueColor: pct >= (th.danger ?? 90) ? Tokens.color("meter", "high")
        : pct >= (th.warn ?? 50) ? Tokens.color("meter", "mid")
        : Tokens.color("bar", "foreground")
}

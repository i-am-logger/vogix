// Swap-in-use cell; hidden on hosts with no swap at all. Swap filling is
// an early-warning signal, hence the low thresholds.
import QtQuick
import qs.Bar.widgets
import qs.Services
import qs.Vogix

StatCell {
    readonly property int pct: Math.round(SysStat.swap * 100)
    readonly property var th: ((Config.doc.meters ?? {}).thresholds ?? {}).swap ?? ({})

    visible: SysStat.hasSwap
    label: "SWP"
    value: pct + "%"
    widestValue: "100%"
    valueColor: pct >= (th.danger ?? 80) ? Tokens.color("meter", "high")
        : pct >= (th.warn ?? 20) ? Tokens.color("meter", "mid")
        : Tokens.color("bar", "foreground")
}

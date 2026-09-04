// GPU busy cell — gauge over number over trace, exactly the CPU cell's
// shape, sitting right below it. Absent on hosts without the sysfs knob.
import QtQuick
import qs.Bar.widgets
import qs.Services
import qs.Vogix

StatCell {
    readonly property int pct: Math.round(SysStat.gpuBusy * 100)

    visible: SysStat.hasGpu
    title: "GPU"
    value: String(pct).padStart(3, "0") + "%"
    widestValue: "000%"
    meterValue: SysStat.gpuBusy
    meterWarnAt: 0.6
    meterDangerAt: 0.9
    traceValues: SysStat.gpuHistory
    valueColor: pct >= 90 ? Tokens.color("meter", "high")
        : pct >= 60 ? Tokens.color("meter", "mid")
        : Tokens.color("bar", "foreground")
}

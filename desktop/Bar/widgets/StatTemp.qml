// CPU temperature cell; absent entirely on boards with no usable hwmon
// sensor (SysStat.hasTemp degrade).
import QtQuick
import qs.Bar.widgets
import qs.Services
import qs.Vogix

StatCell {
    readonly property int degrees: Math.round(SysStat.cpuTempC)
    readonly property var th: ((Config.doc.meters ?? {}).thresholds ?? {}).cpuTemp ?? ({})

    visible: SysStat.hasTemp
    title: "TEMP"
    value: String(degrees).padStart(3, "0") + "C"
    widestValue: "000C"
    // Bar then number: die temperature on a 0–100°C gauge.
    meterValue: Math.min(1, SysStat.cpuTempC / 100)
    meterWarnAt: (th.warn ?? 60) / 100
    meterDangerAt: (th.danger ?? 85) / 100
    valueColor: degrees >= (th.danger ?? 85) ? Tokens.color("meter", "high")
        : degrees >= (th.warn ?? 60) ? Tokens.color("meter", "mid")
        : Tokens.color("bar", "foreground")
}

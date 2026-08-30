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
    label: "TEMP"
    value: degrees + "°"
    widestValue: "100°"
    valueColor: degrees >= (th.danger ?? 85) ? Tokens.color("meter", "high")
        : degrees >= (th.warn ?? 60) ? Tokens.color("meter", "mid")
        : Tokens.color("bar", "foreground")
}

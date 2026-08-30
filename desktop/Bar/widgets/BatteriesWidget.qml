pragma ComponentBehavior: Bound
// The BAT panel: every battery UPower knows (internal + peripherals),
// one terse MODEL + percent row each — absent entirely on a batteryless
// desktop. Click opens the power panel.
import QtQuick
import Quickshell.Services.UPower
import qs.Bar.widgets
import qs.Services
import qs.Vogix

FrameCell {
    id: root

    readonly property var cells:
        [...UPower.devices.values].filter(d => d.isLaptopBattery || d.type === UPowerDeviceType.Battery)

    visible: cells.length > 0
    title: "BAT"
    padH: 6
    padV: 4

    Column {
        spacing: 2

        Repeater {
            model: root.cells

            BarText {
                id: row

                required property var modelData
                readonly property int pct: Math.round(row.modelData.percentage * 100)

                text: (row.modelData.model || "BAT").slice(0, 3).toUpperCase() + " " + row.pct
                font.pixelSize: Metrics.micro
                font.letterSpacing: 0.5
                color: row.pct <= 15 ? Tokens.color("bar", "urgent")
                    : (row.pct >= 60 ? Tokens.color("meter", "low") : Tokens.color("bar", "foreground"))
            }
        }
    }

    interactive: true
    onClicked: Panels.toggle("power")
}

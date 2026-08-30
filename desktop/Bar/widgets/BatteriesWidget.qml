// Every battery UPower knows (internal + peripherals), one chip each —
// nothing rendered on a batteryless desktop. Click opens the power panel.
import QtQuick
import QtQuick.Layouts
import Quickshell.Services.UPower
import qs.Bar.widgets
import qs.Services
import qs.Vogix

GridLayout {
    id: root

    property BarAxis axis: null

    flow: (axis?.vertical ?? false) ? GridLayout.TopToBottom : GridLayout.LeftToRight
    rowSpacing: 4
    columnSpacing: 6

    Repeater {
        model: [...UPower.devices.values].filter(d => d.isLaptopBattery || d.type === UPowerDeviceType.Battery)

        BarText {
            id: cell

            required property var modelData
            readonly property int pct: Math.round(cell.modelData.percentage * 100)

            text: cell.pct + "%"
            font.pixelSize: Metrics.caption
            color: cell.pct <= 15 ? Tokens.color("bar", "urgent") : Tokens.color("bar", "muted")

            MouseArea {
                anchors.fill: parent
                onClicked: Panels.toggle("power")
            }
        }
    }
}

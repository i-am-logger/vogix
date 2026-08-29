pragma ComponentBehavior: Bound
// Backlight slider (brightnessctl) + the connected displays.
import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import Quickshell
import qs.Panels
import qs.Services
import qs.Vogix

ColumnLayout {
    Component.onCompleted: Brightness.refresh()

    spacing: 10

    PanelLabel {
        text: "Monitors"
        font.bold: true
        color: Tokens.color("popup", "accent")
    }

    RowLayout {
        Layout.fillWidth: true
        visible: Brightness.level >= 0
        spacing: 10

        PanelLabel {
            text: "󰃟"
        }

        Slider {
            Layout.fillWidth: true
            from: 0.01
            to: 1
            value: Brightness.level < 0 ? 1 : Brightness.level
            onMoved: Brightness.set(value)
        }

        PanelLabel {
            text: Math.round(Math.max(0, Brightness.level) * 100) + "%"
        }
    }

    Repeater {
        model: Quickshell.screens

        PanelLabel {
            id: screenRow

            required property var modelData

            Layout.fillWidth: true
            text: "󰍹 " + screenRow.modelData.name + "  " + screenRow.modelData.width
                + "×" + screenRow.modelData.height + " @" + screenRow.modelData.devicePixelRatio + "x"
            color: Tokens.color("popup", "muted")
        }
    }
}

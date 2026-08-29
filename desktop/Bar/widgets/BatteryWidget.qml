// Battery percent + charge state (hidden on desktops); click opens the
// power panel. Urgent color under 15% on battery.
import QtQuick
import qs.Bar.widgets
import qs.Services
import qs.Vogix

BarText {
    visible: Battery.present
    readonly property int pct: Math.round(Battery.percentage * 100)

    text: {
        const icons = ["󰁺", "󰁻", "󰁼", "󰁽", "󰁾", "󰁿", "󰂀", "󰂁", "󰂂", "󰁹"];
        const icon = Battery.charging ? "󰂄" : icons[Math.min(9, Math.floor(pct / 10))];
        return icon + " " + pct + "%";
    }
    color: !Battery.charging && pct <= 15
        ? Tokens.color("bar", "urgent")
        : Tokens.color("bar", "foreground")

    MouseArea {
        anchors.fill: parent
        onClicked: Panels.toggle("power")
    }
}

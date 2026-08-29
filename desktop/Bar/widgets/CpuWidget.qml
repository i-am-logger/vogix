// CPU load gauge, urgent when pinned.
import QtQuick
import qs.Bar.widgets
import qs.Services
import qs.Vogix

BarText {
    readonly property int pct: Math.round(SysStat.cpu * 100)

    text: "󰻠 " + pct + "%"
    color: pct >= 90
        ? Tokens.color("bar", "urgent")
        : Tokens.color("bar", "foreground")
}

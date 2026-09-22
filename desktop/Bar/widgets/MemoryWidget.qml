// Memory-in-use gauge, urgent when nearly full.
import QtQuick
import qs.Bar.widgets
import qs.Services
import qs.Vogix

BarText {
    id: root

    readonly property int pct: Math.round(SysStat.memory * 100)

    text: "󰍛 " + pct + "%"
    color: pct >= 90
        ? Tokens.color("bar", "urgent")
        : Tokens.color("bar", "foreground")

    // Held while this bar is on screen.
    Lease {
        active: root.axis?.live ?? false
        onAcquire: SysStat.acquire(["memory"])
        onRelease: SysStat.release(["memory"])
    }
}

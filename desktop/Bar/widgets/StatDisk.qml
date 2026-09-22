// Root-filesystem usage cell (30 s df). Fixed 80/95 thresholds — a
// filling disk is urgent at the same point on every host.
import QtQuick
import qs.Bar.widgets
import qs.Services
import qs.Vogix

StatCell {
    id: root

    readonly property int pct: Math.round(SysStat.disk * 100)

    title: "DISK"
    value: String(pct).padStart(3, "0") + "%"
    widestValue: "000%"
    meterValue: SysStat.disk
    meterWarnAt: 0.8
    meterDangerAt: 0.95
    valueColor: pct >= 95 ? Tokens.color("meter", "high")
        : pct >= 80 ? Tokens.color("meter", "mid")
        : Tokens.color("bar", "foreground")

    // Held while this bar is on screen.
    Lease {
        active: root.axis?.live ?? false
        onAcquire: SysStat.acquire(["mounts"])
        onRelease: SysStat.release(["mounts"])
    }
}

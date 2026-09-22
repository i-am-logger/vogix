// CPU load history — untitled: it sits beside the CPU stat cell.
import QtQuick
import qs.Bar.widgets
import qs.Services

GraphCell {
    id: root

    values: SysStat.cpuHistory

    // Held while this bar is on screen.
    Lease {
        active: root.axis?.live ?? false
        onAcquire: SysStat.acquire(["cpu"])
        onRelease: SysStat.release(["cpu"])
    }
}

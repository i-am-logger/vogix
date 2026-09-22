// Memory history — untitled beside the MEM stat cell; link-colored.
import QtQuick
import qs.Bar.widgets
import qs.Services
import qs.Vogix

GraphCell {
    id: root

    values: SysStat.memoryHistory
    lineColor: Theme.semantic.link ?? Tokens.color("bar", "accent")

    // Held while this bar is on screen.
    Lease {
        active: root.axis?.live ?? false
        onAcquire: SysStat.acquire(["memory"])
        onRelease: SysStat.release(["memory"])
    }
}

pragma Singleton
// CPU and memory gauges from /proc, sampled on a slow timer — cheap enough
// to always run while any widget shows them.
import QtQuick
import Quickshell
import Quickshell.Io

Singleton {
    id: root

    property real cpu: 0        // 0..1
    property real memory: 0     // 0..1

    property real lastTotal: 0
    property real lastIdle: 0

    Timer {
        interval: 3000
        running: true
        repeat: true
        triggeredOnStart: true
        onTriggered: {
            statFile.reload();
            memFile.reload();
        }
    }

    FileView {
        id: statFile
        path: "/proc/stat"
        watchChanges: false
        preload: false
        onLoaded: {
            const parts = text().split("\n")[0].trim().split(/\s+/).slice(1).map(Number);
            const idle = parts[3] + (parts[4] ?? 0);
            const total = parts.reduce((a, b) => a + b, 0);
            const dTotal = total - root.lastTotal;
            const dIdle = idle - root.lastIdle;
            if (root.lastTotal > 0 && dTotal > 0)
                root.cpu = Math.max(0, Math.min(1, 1 - dIdle / dTotal));
            root.lastTotal = total;
            root.lastIdle = idle;
        }
    }

    FileView {
        id: memFile
        path: "/proc/meminfo"
        watchChanges: false
        preload: false
        onLoaded: {
            const t = text();
            const total = Number((t.match(/MemTotal:\s+(\d+)/) ?? [0, 0])[1]);
            const avail = Number((t.match(/MemAvailable:\s+(\d+)/) ?? [0, 0])[1]);
            if (total > 0)
                root.memory = Math.max(0, Math.min(1, 1 - avail / total));
        }
    }
}

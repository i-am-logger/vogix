pragma Singleton
// System gauges from /proc and /sys, sampled on a slow timer — cheap
// enough to always run while any widget shows them. Each stat also keeps
// a ring buffer (meters.history samples) for the HUD graphs; buffers are
// reassigned whole so Canvas bindings repaint. Filesystem usage is a
// 30 s df over the configured mount points; the CPU temperature source
// is enumerated ONCE from hwmon by driver priority, and hasTemp degrades
// the widgets rather than erroring on boards that expose none.
import QtQuick
import Quickshell
import Quickshell.Io
import qs.Vogix

Singleton {
    id: root

    readonly property int histLen: (Config.doc.meters ?? {}).history ?? 64
    readonly property int sampleMs: (Config.doc.meters ?? {}).sampleMs ?? 100

    property real cpu: 0        // 0..1
    property real memory: 0     // 0..1
    property real swap: 0       // 0..1
    property bool hasSwap: false
    property bool hasTemp: false
    property real cpuTempC: 0
    property real netRxRate: 0  // bytes/s
    property real netTxRate: 0  // bytes/s

    property bool hasGpu: false
    property real gpuBusy: 0    // 0..1
    property real diskIoRate: 0 // bytes/s, reads+writes
    property real uptimeSec: 0
    // gauge name → device basename, and gauge name → live bytes/s; a
    // gauge with no block device is simply absent from both.
    property var gaugeDevice: ({})
    property var gaugeIo: ({})
    property var _lastDevSectors: ({})

    // The capacity gauges the bar may show, in the order meters.mounts
    // names them: absolute mount points, measured by df, plus the literal
    // "swap", which answers from the meminfo figures sampled above rather
    // than probing the same numbers a second way.
    readonly property list<string> gaugePoints: (Config.doc.meters ?? {}).mounts ?? ["/"]

    // The df targets. "/" is always among them — `disk` derives from it
    // whether or not the config asks for a root gauge.
    readonly property list<string> mountPoints: {
        const out = ["/"];
        for (let i = 0; i < root.gaugePoints.length; i++) {
            const p = root.gaugePoints[i];
            if (p.startsWith("/") && !out.includes(p))
                out.push(p);
        }
        return out;
    }

    // Used fraction per mount point, 0..1, replaced WHOLE each df so a
    // mount that goes away goes away. A configured path this host does
    // not mount has NO key here: that absence is the whole point, since a
    // 0% row would read as an empty filesystem instead of no filesystem.
    property var mountUsage: ({})

    // The root row, kept as its own name because the DISK cell predates
    // the configured table and still reads it.
    readonly property real disk: root.mountUsage["/"] ?? 0  // 0..1

    // gaugePoints minus what this host does not have. This tests hasSwap
    // directly instead of calling gaugeUsed, because the swap FRACTION
    // moves on the fast tick and a model rebuilt ten times a second would
    // recreate every cell bound to it.
    readonly property list<string> gaugesPresent: {
        const out = [];
        for (let i = 0; i < root.gaugePoints.length; i++) {
            const p = root.gaugePoints[i];
            if (p === "swap" ? root.hasSwap : root.mountUsage[p] !== undefined)
                out.push(p);
        }
        return out;
    }

    property list<real> cpuHistory: []
    property list<real> memoryHistory: []
    property list<real> netRxHistory: []  // raw bytes/s — graphs normalize
    property list<real> netTxHistory: []
    property list<real> diskIoHistory: [] // raw bytes/s
    property list<real> gpuHistory: []

    property real lastTotal: 0
    property real lastIdle: 0
    property real lastRx: 0
    property real lastTx: 0
    property real lastNetAt: 0
    property real lastDiskSectors: 0
    property real lastDiskAt: 0
    property string tempPath: ""
    property string gpuPath: ""

    // Used fraction of one gauge, or -1 where this host does not have it.
    // Callers render nothing for -1; there is no substitute value that
    // would not claim a filesystem exists.
    function gaugeUsed(point: string): real {
        if (point === "swap")
            return root.hasSwap ? root.swap : -1;
        const used = root.mountUsage[point];
        return used === undefined ? -1 : used;
    }

    function _push(arr, v) {
        const out = arr.length >= root.histLen ? arr.slice(arr.length - root.histLen + 1) : arr.slice();
        out.push(v);
        return out;
    }

    // The fast tick — cpu/mem/net at meters.sampleMs (default 10 Hz), so
    // the graphs move like instruments, not like a status page.
    Timer {
        interval: root.sampleMs
        running: true
        repeat: true
        triggeredOnStart: true
        onTriggered: {
            statFile.reload();
            memFile.reload();
            netFile.reload();
        }
    }

    // The history tick — 1 Hz, so the graphs show a real time window
    // (history samples = seconds) instead of a 6-second blur; the fast
    // tick above keeps the READOUTS live. Disk I/O and GPU sample here
    // too — 1 Hz is their natural rate.
    Timer {
        interval: 1000
        running: true
        repeat: true
        triggeredOnStart: true
        onTriggered: {
            root.cpuHistory = root._push(root.cpuHistory, root.cpu);
            root.memoryHistory = root._push(root.memoryHistory, root.memory);
            root.netRxHistory = root._push(root.netRxHistory, root.netRxRate);
            root.netTxHistory = root._push(root.netTxHistory, root.netTxRate);
            diskstatsFile.reload();
            uptimeFile.reload();
            if (root.gpuPath !== "")
                gpuFile.reload();
        }
    }

    Timer {
        interval: 3000
        running: true
        repeat: true
        onTriggered: {
            if (root.tempPath !== "")
                tempFile.reload();
        }
    }

    Timer {
        interval: 30000
        running: true
        repeat: true
        triggeredOnStart: true
        onTriggered: diskProc.running = true
    }

    FileView {
        id: statFile
        path: "/proc/stat"
        watchChanges: false
        preload: true
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
        preload: true
        onLoaded: {
            const t = text();
            const total = Number((t.match(/MemTotal:\s+(\d+)/) ?? [0, 0])[1]);
            const avail = Number((t.match(/MemAvailable:\s+(\d+)/) ?? [0, 0])[1]);
            if (total > 0)
                root.memory = Math.max(0, Math.min(1, 1 - avail / total));
            const swapTotal = Number((t.match(/SwapTotal:\s+(\d+)/) ?? [0, 0])[1]);
            const swapFree = Number((t.match(/SwapFree:\s+(\d+)/) ?? [0, 0])[1]);
            root.hasSwap = swapTotal > 0;
            root.swap = swapTotal > 0
                ? Math.max(0, Math.min(1, 1 - swapFree / swapTotal))
                : 0;
        }
    }

    FileView {
        id: netFile
        path: "/proc/net/dev"
        watchChanges: false
        preload: true
        onLoaded: {
            let rx = 0;
            let tx = 0;
            for (const line of text().split("\n").slice(2)) {
                const m = line.trim().match(/^([^:]+):\s*(.*)$/);
                if (!m || m[1] === "lo")
                    continue;
                const f = m[2].trim().split(/\s+/).map(Number);
                rx += f[0];
                tx += f[8];
            }
            const now = Date.now();
            const dt = (now - root.lastNetAt) / 1000;
            if (root.lastNetAt > 0 && dt > 0 && rx >= root.lastRx) {
                root.netRxRate = (rx - root.lastRx) / dt;
                root.netTxRate = (tx - root.lastTx) / dt;
            }
            root.lastRx = rx;
            root.lastTx = tx;
            root.lastNetAt = now;
        }
    }

    // Per-mount usage. The TARGET column comes back with the percentage
    // because it is the only honest presence test: df drops a path that
    // does not exist, and answers with the containing filesystem for one
    // that exists but is not a mount point (a plain /persist directory
    // reports "/"), so a row counts only when it names the path that was
    // asked for. The paths go in as ARGUMENTS rather than interpolated
    // into the script, and stderr is dropped because a mount this host
    // lacks is the expected case, not a fault.
    Process {
        id: diskProc
        command: {
            const argv = ["sh", "-c", "df --output=pcent,target \"$@\" 2>/dev/null", "df"];
            for (let i = 0; i < root.mountPoints.length; i++)
                argv.push(root.mountPoints[i]);
            return argv;
        }

        stdout: StdioCollector {
            onStreamFinished: {
                const seen = {};
                for (const line of text.split("\n")) {
                    const m = line.match(/^\s*(\d+)%\s+(\S.*)$/);
                    if (m && root.mountPoints.includes(m[2]))
                        seen[m[2]] = Math.max(0, Math.min(1, Number(m[1]) / 100));
                }
                // Not one row means df itself failed, since "/" is always
                // asked for and always answers. Keep the last reading
                // rather than publish an empty table, which would read as
                // every mount at 0% instead of as no measurement.
                if (Object.keys(seen).length > 0)
                    root.mountUsage = seen;
            }
        }
    }

    FileView {
        id: uptimeFile
        path: "/proc/uptime"
        watchChanges: false
        preload: true
        onLoaded: root.uptimeSec = Number(text().split(" ")[0])
    }

    // Disk THROUGHPUT (the usage fraction is `disk`): whole physical
    // devices only — the partition rows would double-count every byte.
    FileView {
        id: diskstatsFile
        path: "/proc/diskstats"
        watchChanges: false
        preload: true
        onLoaded: {
            // Whole physical devices feed the TOTAL I/O instrument; the
            // per-gauge devices (partitions, zram) each get their own rate
            // so a mount cell can show what ITS filesystem is doing.
            const wanted = Object.values(root.gaugeDevice);
            const perDev = {};
            let sectors = 0;
            for (const line of text().split("\n")) {
                const f = line.trim().split(/\s+/);
                if (f.length < 11)
                    continue;
                const s = Number(f[5]) + Number(f[9]);
                if (/^(nvme\d+n\d+|sd[a-z]+|vd[a-z]+)$/.test(f[2]))
                    sectors += s;
                if (wanted.includes(f[2]))
                    perDev[f[2]] = s;
            }
            const now = Date.now();
            const dt = (now - root.lastDiskAt) / 1000;
            if (root.lastDiskAt > 0 && dt > 0 && sectors >= root.lastDiskSectors) {
                root.diskIoRate = (sectors - root.lastDiskSectors) * 512 / dt;
                root.diskIoHistory = root._push(root.diskIoHistory, root.diskIoRate);

                const io = {};
                for (const [name, dev] of Object.entries(root.gaugeDevice)) {
                    const cur = perDev[dev];
                    const prev = root._lastDevSectors[dev];
                    if (cur !== undefined && prev !== undefined && cur >= prev)
                        io[name] = (cur - prev) * 512 / dt;
                }
                root.gaugeIo = io;
            }
            root._lastDevSectors = perDev;
            root.lastDiskSectors = sectors;
            root.lastDiskAt = now;
        }
    }

    // One-shot mount→device map for the per-gauge I/O rates. Only rows
    // backed by a real block device count: tmpfs mounts have no I/O and
    // must read ABSENT, and a btrfs subvolume source keeps its device
    // once the [subvol] suffix is stripped.
    Process {
        id: mountDevProc
        running: true
        command: ["sh", "-c", "findmnt -rn -o TARGET,SOURCE; echo '--swaps--'; tail -n +2 /proc/swaps 2>/dev/null | cut -d' ' -f1"]

        stdout: StdioCollector {
            onStreamFinished: {
                const map = {};
                let inSwaps = false;
                for (const line of text.trim().split("\n")) {
                    if (line === "--swaps--") {
                        inSwaps = true;
                        continue;
                    }
                    if (inSwaps) {
                        const dev = line.trim();
                        if (dev.startsWith("/dev/"))
                            map["swap"] = dev.slice(5);
                        continue;
                    }
                    const parts = line.split(" ");
                    if (parts.length >= 2 && parts[1].startsWith("/dev/"))
                        map[parts[0]] = parts[1].slice(5).replace(/\[.*$/, "");
                }
                root.gaugeDevice = map;
            }
        }
    }

    // One-shot GPU enumeration: the first card exposing amdgpu/intel's
    // busy percent; hasGpu degrades the widget away otherwise.
    Process {
        id: gpuProbeProc
        running: true
        command: ["sh", "-c", "for f in /sys/class/drm/card*/device/gpu_busy_percent; do [ -f \"$f\" ] && { echo \"$f\"; break; }; done"]

        stdout: StdioCollector {
            onStreamFinished: {
                const p = text.trim();
                if (p !== "") {
                    root.gpuPath = p;
                    root.hasGpu = true;
                }
            }
        }
    }

    FileView {
        id: gpuFile
        path: root.gpuPath
        watchChanges: false
        preload: true
        onLoaded: {
            root.gpuBusy = Math.max(0, Math.min(1, Number(text().trim()) / 100));
            root.gpuHistory = root._push(root.gpuHistory, root.gpuBusy);
        }
        onLoadFailed: root.hasGpu = false
    }

    // One-shot hwmon enumeration, by driver priority: the CPU die sensor
    // first (k10temp/zenpower on AMD, coretemp on Intel), the SoC thermal
    // zone next, ACPI last.
    Process {
        id: hwmonProc
        running: true
        command: ["sh", "-c", "for d in /sys/class/hwmon/hwmon*; do [ -f \"$d/name\" ] && printf '%s %s\\n' \"$d\" \"$(cat \"$d/name\")\"; done"]

        stdout: StdioCollector {
            onStreamFinished: {
                const byName = {};
                for (const line of text.trim().split("\n")) {
                    const cut = line.indexOf(" ");
                    if (cut > 0)
                        byName[line.slice(cut + 1)] = line.slice(0, cut);
                }
                for (const name of ["k10temp", "zenpower", "coretemp", "cpu_thermal", "acpitz"]) {
                    if (byName[name] !== undefined) {
                        root.tempPath = byName[name] + "/temp1_input";
                        root.hasTemp = true;
                        return;
                    }
                }
            }
        }
    }

    FileView {
        id: tempFile
        path: root.tempPath
        watchChanges: false
        preload: true
        onLoaded: root.cpuTempC = Number(text().trim()) / 1000
        onLoadFailed: root.hasTemp = false
    }
}

pragma Singleton
pragma ComponentBehavior: Bound
// System gauges from /proc and /sys. Every sampler runs only while a
// widget on a visible bar reads its stat: widgets hold per-stat
// references through a Lease on their bar's `live`, so a stat nobody can
// see is not sampled at all. Each stat also keeps a ring buffer
// (meters.history samples) for the HUD graphs; buffers are reassigned
// whole so Canvas bindings repaint. Filesystem usage is a 30 s df over
// the configured mount points; the CPU temperature source is enumerated
// ONCE from hwmon by driver priority, and hasTemp degrades the widgets
// rather than erroring on boards that expose none.
import QtQuick
import Quickshell
import Quickshell.Io
import qs.Vogix
import "lib/blockdev.js" as BlockDev
import "lib/fans.js" as Fans
import "lib/gpu.js" as Gpu

Singleton {
    id: root

    readonly property int histLen: (Config.doc.meters ?? {}).history ?? 64
    readonly property int sampleMs: (Config.doc.meters ?? {}).sampleMs ?? 100

    // A busy counter is a near-instantaneous reading: at idle, amdgpu's
    // gpu_busy_percent swings between single digits and 100 from one read
    // to the next. The published figure is the mean of the last
    // gpuMeanSamples 1 Hz samples, so readout, threshold color and trace
    // show the load level instead of the sampling noise.
    readonly property int gpuMeanSamples: 5
    property list<real> _gpuSamples: []

    // The GPU the cell measures, chosen once at startup from the DRM
    // cards (data/gpu-probe.sh evidence, lib/gpu.js policy): nvidia-smi
    // streaming, amdgpu's busy percent, or Intel's idle residency. None
    // hides the GPU widgets.
    property int gpuSource: Gpu.Source.None
    property string gpuPath: ""
    property string gpuPci: ""
    readonly property bool hasGpu: root.gpuSource !== Gpu.Source.None
    // The previous idle-residency reading and when it was taken.
    property real _gpuIdleMs: -1
    property real _gpuIdleAt: 0

    // References per stat, taken and dropped through acquire/release by
    // the names below. A stat's samplers run only while it has one; when
    // the last goes, its histories and rate baselines are dropped, so a
    // resumed graph restarts instead of splicing across the gap.
    property int cpuRefs: 0
    property int memoryRefs: 0  // memory and swap
    property int netRefs: 0
    property int diskRefs: 0    // disk throughput, total and per gauge
    property int gpuRefs: 0
    property int uptimeRefs: 0
    property int tempRefs: 0
    property int fansRefs: 0
    property int mountsRefs: 0  // the df gauges (mountUsage, disk)

    readonly property bool cpuWanted: root.cpuRefs > 0
    readonly property bool memoryWanted: root.memoryRefs > 0
    readonly property bool netWanted: root.netRefs > 0
    readonly property bool diskWanted: root.diskRefs > 0
    readonly property bool gpuWanted: root.gpuRefs > 0
    readonly property bool uptimeWanted: root.uptimeRefs > 0
    readonly property bool tempWanted: root.tempRefs > 0
    readonly property bool fansWanted: root.fansRefs > 0
    readonly property bool mountsWanted: root.mountsRefs > 0

    function acquire(stats: list<string>): void {
        root._count(stats, 1);
    }

    function release(stats: list<string>): void {
        root._count(stats, -1);
    }

    function _count(stats: list<string>, delta: int): void {
        for (const s of stats) {
            switch (s) {
            case "cpu":
                root.cpuRefs = Math.max(0, root.cpuRefs + delta);
                break;
            case "memory":
                root.memoryRefs = Math.max(0, root.memoryRefs + delta);
                break;
            case "net":
                root.netRefs = Math.max(0, root.netRefs + delta);
                break;
            case "disk":
                root.diskRefs = Math.max(0, root.diskRefs + delta);
                break;
            case "gpu":
                root.gpuRefs = Math.max(0, root.gpuRefs + delta);
                break;
            case "uptime":
                root.uptimeRefs = Math.max(0, root.uptimeRefs + delta);
                break;
            case "temp":
                root.tempRefs = Math.max(0, root.tempRefs + delta);
                break;
            case "fans":
                root.fansRefs = Math.max(0, root.fansRefs + delta);
                break;
            case "mounts":
                root.mountsRefs = Math.max(0, root.mountsRefs + delta);
                break;
            default:
                console.error("vogix: SysStat has no stat named '" + s + "'");
            }
        }
    }

    // The stats being sampled, comma-joined ("none" when idle).
    function status(): string {
        const on = [];
        if (root.cpuWanted)
            on.push("cpu");
        if (root.memoryWanted)
            on.push("memory");
        if (root.netWanted)
            on.push("net");
        if (root.diskWanted)
            on.push("disk");
        if (root.gpuWanted)
            on.push("gpu");
        if (root.uptimeWanted)
            on.push("uptime");
        if (root.tempWanted)
            on.push("temp");
        if (root.fansWanted)
            on.push("fans");
        if (root.mountsWanted)
            on.push("mounts");
        return on.length > 0 ? on.join(",") : "none";
    }

    onCpuWantedChanged: {
        if (!cpuWanted) {
            lastTotal = 0;
            lastIdle = 0;
            _cpuFresh = false;
            cpuHistory = [];
        }
    }

    onMemoryWantedChanged: {
        if (!memoryWanted) {
            _memoryFresh = false;
            memoryHistory = [];
        }
    }

    onNetWantedChanged: {
        if (!netWanted) {
            lastNetAt = 0;
            _netFresh = false;
            netRxHistory = [];
            netTxHistory = [];
        }
    }

    onDiskWantedChanged: {
        if (!diskWanted) {
            lastDiskAt = 0;
            _lastDevSectors = {};
            diskIoHistory = [];
        }
    }

    onGpuWantedChanged: {
        if (!gpuWanted) {
            _gpuSamples = [];
            _gpuIdleMs = -1;
            _gpuIdleAt = 0;
            gpuHistory = [];
        }
    }

    // Whether a sample exists since the stat was last acquired: the
    // history tick records real samples only, never a stale or zero one.
    property bool _cpuFresh: false
    property bool _memoryFresh: false
    property bool _netFresh: false

    property real cpu: 0        // 0..1
    property real memory: 0     // 0..1
    property real swap: 0       // 0..1
    property bool hasSwap: false
    property bool hasTemp: false
    property real cpuTempC: 0
    property real netRxRate: 0  // bytes/s
    property real netTxRate: 0  // bytes/s

    // hwmon fans (data/fan-probe.sh, lib/fans.js), enumerated once:
    // every tachometer, the latest RPM per fan, and the fans that have
    // spun this session — the cells list only those, since an empty
    // header reads 0 forever. fansPresent is reassigned only when a fan
    // first spins, so the cells are not rebuilt on every reading.
    property var fans: []
    property var fanRpm: ({})
    property list<string> fansPresent: []
    property var _fanSeen: ({})

    property real gpuBusy: 0    // 0..1
    property real diskIoRate: 0 // bytes/s, reads+writes
    property real uptimeSec: 0
    // gauge name → kernel device name (as /proc/diskstats names it), and
    // gauge name → live bytes/s; a gauge with no block device is simply
    // absent from both.
    property var gaugeDevice: ({})
    property var gaugeIo: ({})
    property var _lastDevSectors: ({})
    // The whole physical disks the total I/O sums, and whether the
    // one-shot device probe has answered yet.
    property list<string> _physicalDisks: []
    property bool _blockProbed: false

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

    // "/" is RAM-backed (tmpfs/ramfs): the impermanence layout, where the
    // root is per-boot scratch and the storage lives on the mounts listed
    // beside it. The root GAUGE is omitted then; `disk` still reads it.
    property bool rootInMemory: false

    // gaugePoints minus what this host does not have. This tests hasSwap
    // directly instead of calling gaugeUsed, because the swap FRACTION
    // moves on the fast tick and a model rebuilt ten times a second would
    // recreate every cell bound to it.
    readonly property list<string> gaugesPresent: {
        const out = [];
        for (let i = 0; i < root.gaugePoints.length; i++) {
            const p = root.gaugePoints[i];
            const present = p === "swap" ? root.hasSwap
                : p === "/" ? root.mountUsage[p] !== undefined && !root.rootInMemory
                : root.mountUsage[p] !== undefined;
            if (present)
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

    // The probe record for one fan key, or null.
    function fan(key: string): var {
        return root.fans.find(f => f.key === key) ?? null;
    }

    function _fanReading(key: string, text: string): void {
        const v = Fans.rpm(text);
        if (v === null)
            return;
        const next = Object.assign({}, root.fanRpm);
        next[key] = v;
        root.fanRpm = next;
        if (v > 0 && root._fanSeen[key] !== true) {
            const seen = Object.assign({}, root._fanSeen);
            seen[key] = true;
            root._fanSeen = seen;
            root.fansPresent = Fans.spinning(root.fans, seen);
        }
    }

    // A tachometer that stopped being readable (a USB cooler unplugged)
    // leaves the table instead of failing a read every tick.
    function _fanGone(key: string): void {
        console.warn("SysStat: " + key + " is no longer readable; dropping that fan");
        root.fans = root.fans.filter(f => f.key !== key);
        const next = Object.assign({}, root.fanRpm);
        delete next[key];
        root.fanRpm = next;
        root.fansPresent = Fans.spinning(root.fans, root._fanSeen);
    }

    // One raw busy sample, 0..1: folded into the window, published as its
    // mean. A sample that lands after the last reader let go is dropped.
    function _gpuSample(v: real): void {
        if (!root.gpuWanted)
            return;
        root._gpuSamples = Gpu.pushWindow(root._gpuSamples, v, root.gpuMeanSamples);
        root.gpuBusy = Gpu.mean(root._gpuSamples);
        root.gpuHistory = root._push(root.gpuHistory, root.gpuBusy);
    }

    // The fast tick — cpu/mem/net at meters.sampleMs (default 10 Hz), so
    // the graphs move like instruments, not like a status page.
    Timer {
        interval: root.sampleMs
        running: root.cpuWanted || root.memoryWanted || root.netWanted
        repeat: true
        triggeredOnStart: true
        onTriggered: {
            if (root.cpuWanted)
                statFile.reload();
            if (root.memoryWanted)
                memFile.reload();
            if (root.netWanted)
                netFile.reload();
        }
    }

    // The history tick — 1 Hz, so the graphs show a real time window
    // (history samples = seconds) instead of a 6-second blur; the fast
    // tick above keeps the READOUTS live. Disk I/O and the sysfs GPU
    // sources sample here too — 1 Hz is their natural rate, and
    // nvidia-smi streams at the same period.
    Timer {
        interval: 1000
        running: root.cpuWanted || root.memoryWanted || root.netWanted
            || root.diskWanted || root.gpuWanted || root.uptimeWanted
        repeat: true
        triggeredOnStart: true
        onTriggered: {
            if (root._cpuFresh)
                root.cpuHistory = root._push(root.cpuHistory, root.cpu);
            if (root._memoryFresh)
                root.memoryHistory = root._push(root.memoryHistory, root.memory);
            if (root._netFresh) {
                root.netRxHistory = root._push(root.netRxHistory, root.netRxRate);
                root.netTxHistory = root._push(root.netTxHistory, root.netTxRate);
            }
            if (root.diskWanted)
                diskstatsFile.reload();
            if (root.uptimeWanted)
                uptimeFile.reload();
            if (root.gpuWanted && (root.gpuSource === Gpu.Source.BusyPercent || root.gpuSource === Gpu.Source.IdleResidency))
                gpuFile.reload();
        }
    }

    Timer {
        interval: 3000
        running: (root.tempWanted && root.tempPath !== "") || (root.fansWanted && root.fans.length > 0)
        repeat: true
        triggeredOnStart: true
        onTriggered: {
            if (root.tempWanted && root.tempPath !== "")
                tempFile.reload();
            if (root.fansWanted)
                for (const f of fanFiles.instances)
                    f.reload();
        }
    }

    Timer {
        interval: 30000
        running: root.mountsWanted
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
            // The startup preload must not seed a baseline that a later
            // acquire would difference against.
            if (!root.cpuWanted)
                return;
            const parts = text().split("\n")[0].trim().split(/\s+/).slice(1).map(Number);
            const idle = parts[3] + (parts[4] ?? 0);
            const total = parts.reduce((a, b) => a + b, 0);
            const dTotal = total - root.lastTotal;
            const dIdle = idle - root.lastIdle;
            if (root.lastTotal > 0 && dTotal > 0) {
                root.cpu = Math.max(0, Math.min(1, 1 - dIdle / dTotal));
                root._cpuFresh = true;
            }
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
            if (!root.memoryWanted)
                return;
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
            root._memoryFresh = true;
        }
    }

    FileView {
        id: netFile
        path: "/proc/net/dev"
        watchChanges: false
        preload: true
        onLoaded: {
            if (!root.netWanted)
                return;
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
                root._netFresh = true;
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
    // asked for. FSTYPE tells a RAM-backed root apart. The paths go in as
    // ARGUMENTS rather than interpolated into the script, and stderr is
    // dropped because a mount this host lacks is the expected case, not a
    // fault.
    Process {
        id: diskProc
        command: {
            const argv = ["sh", "-c", "df --output=pcent,fstype,target \"$@\" 2>/dev/null", "df"];
            for (let i = 0; i < root.mountPoints.length; i++)
                argv.push(root.mountPoints[i]);
            return argv;
        }

        stdout: StdioCollector {
            onStreamFinished: {
                const seen = {};
                let rootFsType = "";
                for (const line of text.split("\n")) {
                    const m = line.match(/^\s*(\d+)%\s+(\S+)\s+(\S.*)$/);
                    if (!m || !root.mountPoints.includes(m[3]))
                        continue;
                    seen[m[3]] = Math.max(0, Math.min(1, Number(m[1]) / 100));
                    if (m[3] === "/")
                        rootFsType = m[2];
                }
                // Not one row means df itself failed, since "/" is always
                // asked for and always answers. Keep the last reading
                // rather than publish an empty table, which would read as
                // every mount at 0% instead of as no measurement.
                if (Object.keys(seen).length > 0) {
                    root.rootInMemory = rootFsType === "tmpfs" || rootFsType === "ramfs";
                    root.mountUsage = seen;
                }
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
    // disks only for the total — partition and dm rows would count every
    // byte again. Nothing is measured until block-devices.sh has named the
    // disks, so the first reading after it is a baseline, not a spike.
    FileView {
        id: diskstatsFile
        path: "/proc/diskstats"
        watchChanges: false
        preload: true
        onLoaded: {
            if (!root.diskWanted || !root._blockProbed)
                return;
            // Whole physical disks feed the TOTAL I/O instrument; the
            // per-gauge devices (partitions, dm-N, zram) each get their
            // own rate so a mount cell can show what ITS filesystem is
            // doing.
            const now = Date.now();
            const read = BlockDev.sectors(text(), root._physicalDisks, Object.values(root.gaugeDevice));
            const dt = (now - root.lastDiskAt) / 1000;
            if (root.lastDiskAt > 0 && dt > 0 && read.total >= root.lastDiskSectors) {
                root.diskIoRate = (read.total - root.lastDiskSectors) * 512 / dt;
                root.diskIoHistory = root._push(root.diskIoHistory, root.diskIoRate);

                const io = {};
                for (const [name, dev] of Object.entries(root.gaugeDevice)) {
                    const cur = read.perDev[dev];
                    const prev = root._lastDevSectors[dev];
                    if (cur !== undefined && prev !== undefined && cur >= prev)
                        io[name] = (cur - prev) * 512 / dt;
                }
                root.gaugeIo = io;
            }
            root._lastDevSectors = read.perDev;
            root.lastDiskSectors = read.total;
            root.lastDiskAt = now;
        }
    }

    // One-shot block-device map (data/block-devices.sh): the kernel device
    // behind each mount and swap, resolved through its device-node
    // symlinks so a LUKS or LVM mapping reports the dm-N that diskstats
    // counts, and the whole physical disks. Mounts without a block device
    // (tmpfs) have no entry and their I/O reads ABSENT.
    Process {
        id: mountDevProc
        running: true
        command: ["sh", Quickshell.shellDir + "/data/block-devices.sh"]

        stdout: StdioCollector {
            onStreamFinished: {
                const found = BlockDev.parse(text);
                root.gaugeDevice = found.devices;
                root._physicalDisks = found.disks;
                root._blockProbed = true;
            }
        }
    }

    // One-shot GPU enumeration; gpuSource stays None on hosts with no
    // unprivileged busy source (nouveau, or a runtime-suspended dGPU and
    // nothing else), and hasGpu hides the widgets.
    Process {
        id: gpuProbeProc
        running: true
        command: ["sh", Quickshell.shellDir + "/data/gpu-probe.sh"]

        stdout: StdioCollector {
            onStreamFinished: {
                const pick = Gpu.choose(Gpu.parseProbe(text));
                root.gpuPath = pick.path;
                root.gpuPci = pick.pci;
                root.gpuSource = pick.source;
            }
        }
    }

    // The sysfs sources, read on the history tick. The first residency
    // reading only sets the baseline.
    FileView {
        id: gpuFile
        path: root.gpuPath
        watchChanges: false
        preload: true
        onLoaded: {
            // The load on a path change must not seed a residency baseline
            // that a later acquire would difference against.
            if (!root.gpuWanted)
                return;
            if (root.gpuSource === Gpu.Source.BusyPercent) {
                const v = Gpu.busyPercent(text());
                if (v !== null)
                    root._gpuSample(v);
            } else if (root.gpuSource === Gpu.Source.IdleResidency) {
                const idleMs = Number(text().trim());
                const now = Date.now();
                const v = Gpu.residencyBusy(root._gpuIdleMs, idleMs, now - root._gpuIdleAt);
                root._gpuIdleMs = idleMs;
                root._gpuIdleAt = now;
                if (v !== null)
                    root._gpuSample(v);
            }
        }
        onLoadFailed: {
            console.warn("SysStat: " + root.gpuPath + " is unreadable; the GPU cell is off");
            root.gpuSource = Gpu.Source.None;
        }
    }

    // NVIDIA: one long-lived nvidia-smi printing utilization.gpu every
    // second for the chosen device, instead of a process per sample. If it
    // exits or prints something other than a number, the cell goes away
    // rather than showing a stale or invented figure.
    Process {
        id: nvidiaProc
        running: root.gpuWanted && root.gpuSource === Gpu.Source.NvidiaSmi
        command: ["nvidia-smi", "--id=" + root.gpuPci, "--query-gpu=utilization.gpu",
            "--format=csv,noheader,nounits", "--loop-ms=1000"]

        stdout: SplitParser {
            onRead: line => {
                const v = Gpu.nvidiaSample(line);
                if (v === null) {
                    console.warn("SysStat: nvidia-smi reported '" + line.trim() + "'; the GPU cell is off");
                    root.gpuSource = Gpu.Source.None;
                    return;
                }
                root._gpuSample(v);
            }
        }

        onRunningChanged: {
            // Stopped because the last reader let go: not a failure.
            if (running || !root.gpuWanted || root.gpuSource !== Gpu.Source.NvidiaSmi)
                return;
            console.warn("SysStat: nvidia-smi exited; the GPU cell is off");
            root.gpuSource = Gpu.Source.None;
        }
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

    Process {
        id: fanProbeProc
        running: true
        command: ["sh", Quickshell.shellDir + "/data/fan-probe.sh"]

        stdout: StdioCollector {
            onStreamFinished: root.fans = Fans.parseProbe(text)
        }
    }

    // One reader per tachometer, reloaded on the slow tick with the
    // temperature.
    Variants {
        id: fanFiles
        model: root.fans

        FileView {
            required property var modelData

            path: modelData.key
            watchChanges: false
            preload: true
            onLoaded: root._fanReading(modelData.key, text())
            // Deferred: dropping the fan destroys this very reader.
            onLoadFailed: Qt.callLater(root._fanGone, modelData.key)
        }
    }
}

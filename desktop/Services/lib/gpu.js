.pragma library
// GPU busy figure for SysStat: which device the cell measures, and how
// its raw readings become the published 0..1 value. Pure functions; the
// QML side does the I/O.

// Where the busy figure comes from.
var Source = Object.freeze({
    None: 0,
    // `nvidia-smi --query-gpu=utilization.gpu --loop-ms`: percent of its
    // own sample period a kernel was executing, streamed.
    NvidiaSmi: 1,
    // amdgpu's gpu_busy_percent: a near-instantaneous percent.
    BusyPercent: 2,
    // i915 rc6_residency_ms / xe gtidle idle_residency_ms: cumulative ms
    // the GT spent idle. Busy is the complement of the idle share of wall
    // time; it counts time awake-but-idle before the GT drops into RC6,
    // so it bounds engine busyness from above. These are the only GPU
    // activity counters Intel exposes without CAP_PERFMON.
    IdleResidency: 3,
});

// gpu-probe.sh output → one record per DRM card.
function parseProbe(text) {
    const cards = [];
    for (const line of text.split("\n")) {
        const f = line.split("\t");
        if (f.length !== 8)
            continue;
        const v = s => s === "-" ? "" : s;
        cards.push({
            card: f[0],
            driver: v(f[1]),
            control: v(f[2]),
            bootVga: v(f[3]),
            pci: v(f[4]),
            busyPath: v(f[5]),
            idlePath: v(f[6]),
            smi: f[7] === "1",
        });
    }
    return cards;
}

// A GPU that runtime PM may power off (power/control "auto") and that is
// not the boot display device is left alone: sampling it once a second
// would hold a hybrid laptop's idle dGPU awake.
function sampleable(card) {
    return !(card.control === "auto" && card.bootVga !== "1");
}

// The device the cell measures, as { source, path, pci }. A discrete
// NVIDIA GPU that stays powered comes first — on hybrid hosts that keep
// it on (PRIME sync, a MUX in dGPU mode) it is the one rendering. Then
// amdgpu's direct busy figure, then Intel's idle residency.
function choose(cards) {
    const ok = cards.filter(sampleable);
    const nv = ok.find(c => c.driver === "nvidia" && c.smi && c.pci !== "");
    if (nv)
        return { source: Source.NvidiaSmi, path: "", pci: nv.pci };
    const busy = ok.find(c => c.busyPath !== "");
    if (busy)
        return { source: Source.BusyPercent, path: busy.busyPath, pci: busy.pci };
    const idle = ok.find(c => c.idlePath !== "");
    if (idle)
        return { source: Source.IdleResidency, path: idle.idlePath, pci: idle.pci };
    return { source: Source.None, path: "", pci: "" };
}

function clamp01(v) {
    return Math.max(0, Math.min(1, v));
}

// gpu_busy_percent text → 0..1, or null when it is not a number.
function busyPercent(text) {
    const t = text.trim();
    return /^\d+$/.test(t) ? clamp01(Number(t) / 100) : null;
}

// Busy share between two idle-residency readings taken elapsedMs apart,
// or null when there is no interval to measure: the first reading, a
// counter that went backwards (u32 wrap, device reset), or no elapsed
// time.
function residencyBusy(prevIdleMs, idleMs, elapsedMs) {
    if (prevIdleMs < 0 || elapsedMs <= 0 || idleMs < prevIdleMs)
        return null;
    return clamp01(1 - (idleMs - prevIdleMs) / elapsedMs);
}

// One `--format=csv,noheader,nounits` utilization line → 0..1, or null
// for anything else ("[N/A]", "[Not Supported]", an error line).
function nvidiaSample(line) {
    const t = line.trim();
    return /^\d+(\.\d+)?$/.test(t) ? clamp01(Number(t) / 100) : null;
}

// The last `size` entries of `samples` with `value` appended, as a new
// array (list properties repaint only on reassignment).
function pushWindow(samples, value, size) {
    const out = samples.length >= size ? samples.slice(samples.length - size + 1) : samples.slice();
    out.push(value);
    return out;
}

// Arithmetic mean, 0 for no samples.
function mean(samples) {
    if (samples.length === 0)
        return 0;
    let sum = 0;
    for (let i = 0; i < samples.length; i++)
        sum += samples[i];
    return sum / samples.length;
}

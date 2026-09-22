.pragma library
// Filesystem capacity for SysStat's gauges: what a `df --output=pcent,
// fstype,target` run says about the mount points it was asked for, and
// which configured gauges a host has. Pure functions; SysStat.qml runs df.

// df's rows for the asked `points` → { usage: { point: 0..1 },
// rootFsType }. A row counts only when its TARGET is a point that was
// asked for: df drops a path that does not exist, and answers for one
// that exists but is not a mount point with the filesystem containing it
// (a plain /persist directory reports "/"). rootFsType is "" when "/" was
// not reported.
function parseDf(text, points) {
    const usage = {};
    let rootFsType = "";
    for (const line of text.split("\n")) {
        const m = line.match(/^\s*(\d+)%\s+(\S+)\s+(\S.*)$/);
        if (!m || !points.includes(m[3]))
            continue;
        usage[m[3]] = Math.max(0, Math.min(1, Number(m[1]) / 100));
        if (m[3] === "/")
            rootFsType = m[2];
    }
    return { usage: usage, rootFsType: rootFsType };
}

// A root on tmpfs or ramfs is per-boot scratch (the impermanence layout):
// the storage is on the mounts listed beside it.
function inMemory(fsType) {
    return fsType === "tmpfs" || fsType === "ramfs";
}

// The configured gauges this host has, in their configured order: a
// mount point df measured (the root only while it is not RAM-backed),
// and `swap` while swap exists.
function present(gaugePoints, usage, rootInMemory, hasSwap) {
    const out = [];
    for (const p of gaugePoints) {
        const has = p === "swap" ? hasSwap
            : p === "/" ? usage[p] !== undefined && !rootInMemory
            : usage[p] !== undefined;
        if (has)
            out.push(p);
    }
    return out;
}

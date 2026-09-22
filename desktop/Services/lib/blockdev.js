.pragma library
// Block devices for SysStat's disk I/O: which kernel device backs each
// gauge, which devices are whole physical disks, and the sector counts
// /proc/diskstats reports for them. Pure functions; the QML side does the
// I/O.

// findmnt's raw-mode escapes (\x20 for a space) → the path itself.
function unescape(s) {
    return s.replace(/\\x([0-9a-fA-F]{2})/g, (_, h) => String.fromCharCode(parseInt(h, 16)));
}

// block-devices.sh output → { devices: { gauge → kernel name }, disks }.
// Gauges are mount targets plus "swap" (the first swap device); disks
// lists the whole physical disks.
function parse(text) {
    const devices = {};
    const disks = [];
    for (const line of text.split("\n")) {
        const f = line.split("\t");
        if (f[0] === "mount" && f.length === 3)
            devices[unescape(f[1])] = f[2];
        else if (f[0] === "swap" && f.length === 2 && devices["swap"] === undefined)
            devices["swap"] = f[1];
        else if (f[0] === "disk" && f.length === 2)
            disks.push(f[1]);
    }
    return { devices: devices, disks: disks };
}

// /proc/diskstats → { total, perDev }: sectors read plus written, summed
// over the whole disks for the total (partitions and stacked devices
// would count the same bytes again), and per device for each wanted name.
function sectors(text, disks, wanted) {
    const perDev = {};
    let total = 0;
    for (const line of text.split("\n")) {
        const f = line.trim().split(/\s+/);
        if (f.length < 11)
            continue;
        const s = Number(f[5]) + Number(f[9]);
        if (disks.includes(f[2]))
            total += s;
        if (wanted.includes(f[2]))
            perDev[f[2]] = s;
    }
    return { total: total, perDev: perDev };
}

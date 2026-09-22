.pragma library
// hwmon fans for SysStat and the fan cells: what each tachometer is
// called, how its reading scales, and which ones are real. Pure
// functions; the QML side does the I/O.

// fan-probe.sh output → one record per tachometer, keyed by its input
// path. maxRpm is 0 when the chip reports no fanN_max.
function parseProbe(text) {
    const fans = [];
    for (const line of text.split("\n")) {
        const f = line.split("\t");
        if (f.length !== 5)
            continue;
        const max = Number(f[4]);
        fans.push({
            key: f[0],
            chip: f[1] === "-" ? "" : f[1],
            index: f[2],
            label: f[3] === "-" ? "" : f[3].trim(),
            maxRpm: f[4] !== "-" && max > 0 ? max : 0,
        });
    }
    return fans;
}

// A fanN_input reading → RPM, or null when it is not a number.
function rpm(text) {
    const t = text.trim();
    return /^\d+$/.test(t) ? Number(t) : null;
}

// The cell title: the chip's own label for the header, else FAN<n>.
// A rail is narrow, so vertical titles keep four characters, like the
// mount cells do; an unlabelled header past FAN9 shortens to F<n> there,
// so FAN10 never reads as FAN1.
function title(fan, vertical) {
    if (fan.label === "") {
        const t = "FAN" + fan.index;
        return vertical && t.length > 4 ? "F" + fan.index : t;
    }
    const full = fan.label.toUpperCase();
    return vertical ? full.slice(0, 4).replace(/[\s_-]+$/, "") : full;
}

// True when the fans span more than one chip: FAN1 then names two
// different headers, so each cell also shows its chip.
function spansChips(fans) {
    const chips = {};
    for (const f of fans)
        chips[f.chip] = true;
    return Object.keys(chips).length > 1;
}

// The keys, in probe order, of the fans that have spun at least once.
// Boards expose a tachometer per header whether or not anything is
// plugged in, and an empty header reads 0 forever; a fan that stops later
// (a semi-passive GPU fan at idle) stays listed, reading 0.
function spinning(fans, seen) {
    return fans.filter(f => seen[f.key] === true).map(f => f.key);
}

// Gauge fraction: the reading over the chip's reported maximum, or -1
// (number only) when the chip reports none, since no fixed full scale
// fits both a 1200 RPM case fan and a 5000 RPM laptop blower.
function gauge(fan, reading) {
    return fan.maxRpm > 0 ? Math.min(1, reading / fan.maxRpm) : -1;
}

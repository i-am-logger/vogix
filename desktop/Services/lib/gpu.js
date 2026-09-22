.pragma library
// GPU busy figure for SysStat: how raw readings become the published
// 0..1 value. Pure functions; the QML side does the I/O.

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

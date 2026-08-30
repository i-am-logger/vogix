// Disk THROUGHPUT history (usage sits in the DISK stat cell — it barely
// moves; I/O is the live instrument). Normalized to the window's peak.
import QtQuick
import qs.Bar.widgets
import qs.Services
import qs.Vogix

GraphCell {
    function norm(arr: list<real>): list<real> {
        let max = 1;
        for (const v of arr)
            if (v > max)
                max = v;
        return arr.map(v => v / max);
    }

    function fmt(rate: real): string {
        if (rate >= 1024 * 1024)
            return (rate / (1024 * 1024)).toFixed(1) + "M";
        if (rate >= 1024)
            return Math.round(rate / 1024) + "K";
        return Math.round(rate) + "B";
    }

    title: "I/O"
    values: norm(SysStat.diskIoHistory)
    valueText: fmt(SysStat.diskIoRate)
    lineColor: Theme.semantic.notice ?? Tokens.color("bar", "accent")
}

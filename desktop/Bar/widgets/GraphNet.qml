// Network history panel: download solid, upload dashed, both normalized
// to the window's own peak (rates have no natural 100%).
import QtQuick
import qs.Bar.widgets
import qs.Services

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

    title: "NET"
    values: norm(SysStat.netRxHistory)
    secondary: norm(SysStat.netTxHistory)
    valueText: "▼" + fmt(SysStat.netRxRate)
}

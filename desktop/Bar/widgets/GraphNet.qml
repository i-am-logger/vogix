// Network history graph: download solid, upload dashed, both normalized
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

    label: "NET"
    values: norm(SysStat.netRxHistory)
    secondary: norm(SysStat.netTxHistory)
}

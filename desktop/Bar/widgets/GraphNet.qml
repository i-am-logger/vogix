// Network history — untitled beside the NET stat cell (which carries
// the rates). Download solid, upload dashed, both normalized to the
// window's own peak.
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

    values: norm(SysStat.netRxHistory)
    secondary: norm(SysStat.netTxHistory)
}

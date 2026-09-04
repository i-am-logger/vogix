// System uptime cell — how long the machine has been up, in the HUD's
// terse form (2D4H / 6H56M / 12M).
import QtQuick
import qs.Bar.widgets
import qs.Components
import qs.Services
import qs.Vogix

FrameCell {
    id: root

    title: "UP"
    padH: 8
    padV: 3

    function fmt(secs: real): string {
        const mins = Math.floor(secs / 60);
        if (mins >= 60 * 24)
            return Math.floor(mins / (60 * 24)) + "D" + Math.floor((mins % (60 * 24)) / 60) + "H";
        if (mins >= 60)
            return Math.floor(mins / 60) + "H" + (mins % 60) + "M";
        return mins + "M";
    }

    NumericText {
        text: root.fmt(SysStat.uptimeSec)
        widestText: "99D23H"
        font.pixelSize: Metrics.bodySmall
        color: Tokens.color("bar", "foreground")
    }
}

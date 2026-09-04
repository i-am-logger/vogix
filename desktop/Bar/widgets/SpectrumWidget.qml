// The cava spectrum: thin frequency bars. `channel` picks the view —
// "all" is the raw mirrored stereo array (the top-bar mini), "left" and
// "right" are single channels for the bottom bar's far corners, drawn
// with the bass at the OUTER edge so the two mirror each other.
// Vertical rails get band-per-row. Ref-counts the cava subprocess.
import QtQuick
import qs.Bar.widgets
import qs.Services
import qs.Vogix

Canvas {
    id: root

    property BarAxis axis: null
    // Injected by Section; the registry name picks the channel view.
    property string widgetName: ""
    readonly property string channel: widgetName === "spectrum-left" ? "left"
        : widgetName === "spectrum-right" ? "right"
        : "all"
    readonly property bool vertical: axis?.vertical ?? false

    function _view(arr: var): var {
        if (channel === "left")
            return arr.slice(0, arr.length >> 1).reverse();
        if (channel === "right")
            return [...arr.slice(arr.length >> 1)].reverse();
        return arr;
    }

    // The BALLISTIC level, not the instantaneous frame — Cava carries the VU
    // rail's release curve so the bars fall like the meters beside them.
    readonly property var vals: _view(Cava.heldValues.length ? Cava.heldValues : Cava.values)
    readonly property var caps: _view(Cava.capValues)

    // The corner channels draw wide instrument strips; the top-bar mini
    // packs the full mirrored array tighter so it stays a glanceable chip.
    readonly property int barPitch: channel === "all" ? 3 : 4
    readonly property int barSize: channel === "all" ? 2 : 3

    implicitWidth: vertical ? Metrics.body * 2 : Math.max(1, vals.length) * barPitch
    implicitHeight: vertical ? Math.max(1, vals.length) * barPitch : Metrics.body * 1.25

    Component.onCompleted: Cava.acquire()
    Component.onDestruction: Cava.release()

    Connections {
        target: Cava

        function onHeldValuesChanged(): void {
            root.requestPaint();
        }

        function onCapValuesChanged(): void {
            root.requestPaint();
        }
    }

    onWidthChanged: requestPaint()
    onHeightChanged: requestPaint()

    // Positional colouring and a peak cap, the same grammar SegmentedMeter
    // draws the VU with: a bar reads `low` until it passes warnAt, `mid` to
    // dangerAt, `high` above it, and the cap it left behind falls separately.
    // The thresholds are SegmentedMeter's own defaults so a spectrum bar and a
    // VU segment at the same height are the same colour.
    readonly property real warnAt: 0.7
    readonly property real dangerAt: 0.85
    readonly property int capSize: 2

    onPaint: {
        const ctx = getContext("2d");
        ctx.clearRect(0, 0, width, height);
        const vals = root.vals;
        const caps = root.caps;
        const low = Tokens.color("meter", "low");
        const mid = Tokens.color("meter", "mid");
        const high = Tokens.color("meter", "high");
        const cap = Tokens.color("meter", "cap");
        for (let i = 0; i < vals.length; i++) {
            const v = Math.max(0.05, vals[i]);
            ctx.fillStyle = v > root.dangerAt ? high : (v > root.warnAt ? mid : low);
            if (vertical)
                ctx.fillRect(0, i * barPitch, v * width, barSize);
            else
                ctx.fillRect(i * barPitch, height - v * height, barSize, v * height);

            const c = i < caps.length ? caps[i] : -1;
            if (c <= v)
                continue;
            ctx.fillStyle = cap;
            if (vertical)
                ctx.fillRect(Math.max(0, c * width - root.capSize), i * barPitch, root.capSize, barSize);
            else
                ctx.fillRect(i * barPitch, Math.max(0, height - c * height - root.capSize), barSize, root.capSize);
        }
    }
}

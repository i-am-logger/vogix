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

    readonly property var vals: {
        if (channel === "left")
            return Cava.leftValues;
        if (channel === "right")
            return [...Cava.rightValues].reverse();
        return Cava.values;
    }

    readonly property int barPitch: 4
    readonly property int barSize: 3

    implicitWidth: vertical ? Metrics.body * 2 : Math.max(1, vals.length) * barPitch
    implicitHeight: vertical ? Math.max(1, vals.length) * barPitch : Metrics.body * 1.25

    Component.onCompleted: Cava.acquire()
    Component.onDestruction: Cava.release()

    Connections {
        target: Cava

        function onValuesChanged(): void {
            root.requestPaint();
        }
    }

    onWidthChanged: requestPaint()
    onHeightChanged: requestPaint()

    onPaint: {
        const ctx = getContext("2d");
        ctx.clearRect(0, 0, width, height);
        ctx.fillStyle = Tokens.color("bar", "accent");
        const vals = root.vals;
        for (let i = 0; i < vals.length; i++) {
            const v = Math.max(0.05, vals[i]);
            if (vertical)
                ctx.fillRect(0, i * barPitch, v * width, barSize);
            else
                ctx.fillRect(i * barPitch, height - v * height, barSize, v * height);
        }
    }
}

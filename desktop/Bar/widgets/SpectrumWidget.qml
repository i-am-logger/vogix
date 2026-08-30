// The cava spectrum: thin frequency bars, horizontal on the top bar,
// band-per-row on a vertical rail. Ref-counts the cava subprocess so it
// runs only while a spectrum is on screen.
import QtQuick
import qs.Bar.widgets
import qs.Services
import qs.Vogix

Canvas {
    id: root

    property BarAxis axis: null
    readonly property bool vertical: axis?.vertical ?? false

    readonly property int barPitch: 4
    readonly property int barSize: 3

    implicitWidth: vertical ? Metrics.body * 2 : Cava.bars * barPitch
    implicitHeight: vertical ? Cava.bars * barPitch : Metrics.body * 1.25

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
        const vals = Cava.values;
        for (let i = 0; i < vals.length; i++) {
            const v = Math.max(0.05, vals[i]);
            if (vertical)
                ctx.fillRect(0, i * barPitch, v * width, barSize);
            else
                ctx.fillRect(i * barPitch, height - v * height, barSize, v * height);
        }
    }
}

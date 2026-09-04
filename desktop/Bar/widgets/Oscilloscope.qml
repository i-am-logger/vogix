// A real oscilloscope: the sink monitor's waveform (qs.Services.Waveform),
// centered zero line, accent trace. Ref-counts the sample tap so
// capture runs only while this is on screen.
import QtQuick
import qs.Bar.widgets
import qs.Services
import qs.Vogix

FrameCell {
    title: "SCOPE"
    padH: 8
    padV: 3

    // One multiplier for both axes, so the trace keeps its aspect when it is
    // dialled. At 3x the canvas is ~69 px tall, which is why the bottom bar
    // grew to hold it — a scope clipped by its own bar reads as a bug.
    readonly property real scopeScale: 3

    Canvas {
        id: trace

        width: Metrics.body * 8 * parent.scopeScale
        height: Metrics.body * 1.1 * parent.scopeScale

        Connections {
            target: Waveform

            function onWaveformChanged(): void {
                trace.requestPaint();
            }
        }

        onWidthChanged: requestPaint()
        onHeightChanged: requestPaint()

        onPaint: {
            const ctx = getContext("2d");
            ctx.clearRect(0, 0, width, height);
            const wave = Waveform.waveform;
            const mid = height / 2;
            ctx.strokeStyle = Qt.alpha(Tokens.color("meter", "frame"), 0.6);
            ctx.lineWidth = 1;
            ctx.beginPath();
            ctx.moveTo(0, mid);
            ctx.lineTo(width, mid);
            ctx.stroke();
            if (wave.length < 2)
                return;
            const dx = width / (wave.length - 1);
            ctx.strokeStyle = Tokens.color("bar", "accent");
            ctx.beginPath();
            ctx.moveTo(0, mid - wave[0] * mid * 0.9);
            for (let i = 1; i < wave.length; i++)
                ctx.lineTo(i * dx, mid - wave[i] * mid * 0.9);
            ctx.stroke();
        }
    }

    Component.onCompleted: Waveform.acquire()
    Component.onDestruction: Waveform.release()
}

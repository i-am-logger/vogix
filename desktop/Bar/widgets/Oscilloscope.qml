// A real oscilloscope: the sink monitor's waveform (qs.Services.Scope),
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

    Canvas {
        id: trace

        width: Metrics.body * 8
        height: Metrics.body * 1.1

        Connections {
            target: Scope

            function onWaveformChanged(): void {
                trace.requestPaint();
            }
        }

        onWidthChanged: requestPaint()
        onHeightChanged: requestPaint()

        onPaint: {
            const ctx = getContext("2d");
            ctx.clearRect(0, 0, width, height);
            const wave = Scope.waveform;
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

    Component.onCompleted: Scope.acquire()
    Component.onDestruction: Scope.release()
}

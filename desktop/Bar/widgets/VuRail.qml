// The rail's STEREO VU panel: output L/R rising like a console pair,
// output dB below. The mic meter is its own widget (mic-rail), so the
// device pickers can interleave: VU → OUT picker → MIC → IN picker.
import QtQuick
import qs.Bar.widgets
import qs.Components
import qs.Services
import qs.Vogix

FrameCell {
    title: "VU"
    padH: 6
    padV: 6

    Column {
        spacing: Metrics.unit

        Row {
            spacing: Metrics.unit
            anchors.horizontalCenter: parent.horizontalCenter

            RailMeter {
                value: Peaks.outL
                peak: Peaks.outCapL
            }

            RailMeter {
                value: Peaks.outR
                peak: Peaks.outCapR
            }
        }

        NumericText {
            text: Peaks.dbOf(Peaks.outLevel).toFixed(0) + "dB"
            widestText: "-40dB"
            font.pixelSize: Metrics.micro
            color: Tokens.color("meter", "label")
            anchors.horizontalCenter: parent.horizontalCenter
        }
    }

    Component.onCompleted: Peaks.acquire("out")
    Component.onDestruction: Peaks.release("out")
}

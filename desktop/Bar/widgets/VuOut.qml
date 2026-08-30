// Output VU cell: framed OUT title, segmented meter with peak cap, dB
// readout. Ref-counts the peak monitor so pipewire capture runs only
// while this is on screen.
import QtQuick
import qs.Bar.widgets
import qs.Components
import qs.Services
import qs.Vogix

FrameCell {
    title: "OUT"
    padH: 10
    padV: 4

    Row {
        spacing: Metrics.unit * 2

        SegmentedMeter {
            width: Metrics.body * 6
            height: Math.round(Metrics.body * 0.75)
            value: Peaks.outLevel
            peak: Peaks.outCap
            low: Tokens.color("meter", "low")
            mid: Tokens.color("meter", "mid")
            high: Tokens.color("meter", "high")
            unlit: Tokens.color("meter", "unlit")
            capColor: Tokens.color("meter", "cap")
            anchors.verticalCenter: parent.verticalCenter
        }

        NumericText {
            text: Peaks.dbOf(Peaks.outLevel).toFixed(1)
            widestText: "-40.0"
            font.pixelSize: Metrics.caption
            color: Tokens.color("meter", "value")
            anchors.verticalCenter: parent.verticalCenter
        }
    }

    Component.onCompleted: Peaks.acquire("out")
    Component.onDestruction: Peaks.release("out")
}

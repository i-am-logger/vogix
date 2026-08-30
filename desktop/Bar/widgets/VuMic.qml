// Mic VU cell: framed MIC title (urgent while something is actually
// capturing), segmented meter with peak cap, dB readout.
import QtQuick
import qs.Bar.widgets
import qs.Components
import qs.Services
import qs.Vogix

FrameCell {
    title: "MIC"
    titleColor: Privacy.micInUse ? Tokens.color("bar", "urgent") : Tokens.color("meter", "label")
    padH: 10
    padV: 4

    Row {
        spacing: Metrics.unit * 2

        SegmentedMeter {
            width: Metrics.body * 6
            height: Math.round(Metrics.body * 0.75)
            value: Peaks.micLevel
            peak: Peaks.micCap
            low: Tokens.color("meter", "low")
            mid: Tokens.color("meter", "mid")
            high: Tokens.color("meter", "high")
            unlit: Tokens.color("meter", "unlit")
            capColor: Tokens.color("meter", "cap")
            anchors.verticalCenter: parent.verticalCenter
        }

        NumericText {
            text: Peaks.dbOf(Peaks.micLevel).toFixed(1)
            widestText: "-40.0"
            font.pixelSize: Metrics.caption
            color: Tokens.color("meter", "value")
            anchors.verticalCenter: parent.verticalCenter
        }
    }

    Component.onCompleted: Peaks.acquire("mic")
    Component.onDestruction: Peaks.release("mic")
}

// The vertical rail's VU panel: framed, OUT and MIC columns rising like
// a mixing console, output dB below.
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

            SegmentedMeter {
                width: Math.round(Metrics.body * 0.75)
                height: Metrics.body * 6
                vertical: true
                value: Peaks.outLevel
                peak: Peaks.outCap
                low: Tokens.color("meter", "low")
                mid: Tokens.color("meter", "mid")
                high: Tokens.color("meter", "high")
                unlit: Tokens.color("meter", "unlit")
                capColor: Tokens.color("meter", "cap")
            }

            SegmentedMeter {
                width: Math.round(Metrics.body * 0.75)
                height: Metrics.body * 6
                vertical: true
                value: Peaks.micLevel
                peak: Peaks.micCap
                low: Tokens.color("meter", "low")
                mid: Tokens.color("meter", "mid")
                high: Tokens.color("meter", "high")
                unlit: Tokens.color("meter", "unlit")
                capColor: Tokens.color("meter", "cap")
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

    Component.onCompleted: {
        Peaks.acquire("out");
        Peaks.acquire("mic");
    }
    Component.onDestruction: {
        Peaks.release("out");
        Peaks.release("mic");
    }
}

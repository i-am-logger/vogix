// The rail's audio instruments: a STEREO VU panel (output L/R rising
// like a console pair) with the output dB below, and a separate MIC
// panel underneath with the capture level (title lights urgent while
// something is actually recording).
import QtQuick
import qs.Bar.widgets
import qs.Components
import qs.Services
import qs.Vogix

Column {
    spacing: 14

    component RailMeter: SegmentedMeter {
        width: Math.round(Metrics.body * 0.75)
        height: Metrics.body * 6
        vertical: true
        low: Tokens.color("meter", "low")
        mid: Tokens.color("meter", "mid")
        high: Tokens.color("meter", "high")
        unlit: Tokens.color("meter", "unlit")
        capColor: Tokens.color("meter", "cap")
    }

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
    }

    FrameCell {
        title: "MIC"
        titleColor: Privacy.micInUse ? Tokens.color("bar", "urgent") : Tokens.color("meter", "label")
        padH: 6
        padV: 6

        Column {
            spacing: Metrics.unit

            RailMeter {
                value: Peaks.micLevel
                peak: Peaks.micCap
                anchors.horizontalCenter: parent.horizontalCenter
            }

            NumericText {
                text: Peaks.dbOf(Peaks.micLevel).toFixed(0) + "dB"
                widestText: "-40dB"
                font.pixelSize: Metrics.micro
                color: Tokens.color("meter", "label")
                anchors.horizontalCenter: parent.horizontalCenter
            }
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

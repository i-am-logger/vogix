// The rail's MIC panel: capture level with dB below; the title lights
// urgent while something is actually recording. Sits under the OUT
// picker so the IN picker can follow it.
import QtQuick
import qs.Bar.widgets
import qs.Components
import qs.Services
import qs.Vogix

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

    Component.onCompleted: Peaks.acquire("mic")
    Component.onDestruction: Peaks.release("mic")
}

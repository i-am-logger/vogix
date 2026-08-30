// Mic VU: live source peaks with dB readout; the label lights urgent
// while something is actually capturing (the in-use highlight).
import QtQuick
import qs.Bar.widgets
import qs.Components
import qs.Services
import qs.Vogix

VuMeter {
    property BarAxis axis: null

    label: "MIC"
    vertical: axis?.vertical ?? false
    value: Peaks.micLevel
    peak: Peaks.micCap
    db: Peaks.dbOf(Peaks.micLevel)
    low: Tokens.color("meter", "low")
    mid: Tokens.color("meter", "mid")
    high: Tokens.color("meter", "high")
    unlit: Tokens.color("meter", "unlit")
    capColor: Tokens.color("meter", "cap")
    labelColor: Privacy.micInUse ? Tokens.color("bar", "urgent") : Tokens.color("meter", "label")
    valueColor: Tokens.color("meter", "value")

    Component.onCompleted: Peaks.acquire("mic")
    Component.onDestruction: Peaks.release("mic")
}

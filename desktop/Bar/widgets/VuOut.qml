// Output VU: live sink peaks with dB readout. Ref-counts the peak
// monitor so pipewire capture runs only while this is on screen.
import QtQuick
import qs.Bar.widgets
import qs.Components
import qs.Services
import qs.Vogix

VuMeter {
    property BarAxis axis: null

    label: "OUT"
    vertical: axis?.vertical ?? false
    value: Peaks.outLevel
    peak: Peaks.outCap
    db: Peaks.dbOf(Peaks.outLevel)
    low: Tokens.color("meter", "low")
    mid: Tokens.color("meter", "mid")
    high: Tokens.color("meter", "high")
    unlit: Tokens.color("meter", "unlit")
    capColor: Tokens.color("meter", "cap")
    labelColor: Tokens.color("meter", "label")
    valueColor: Tokens.color("meter", "value")

    Component.onCompleted: Peaks.acquire("out")
    Component.onDestruction: Peaks.release("out")
}

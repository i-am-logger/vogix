// The rail meters' shared vertical segmented column (VU and MIC panels).
import QtQuick
import qs.Components
import qs.Vogix

SegmentedMeter {
    width: Math.round(Metrics.body * 0.75)
    height: Metrics.body * 6
    vertical: true
    low: Tokens.color("meter", "low")
    mid: Tokens.color("meter", "mid")
    high: Tokens.color("meter", "high")
    unlit: Tokens.color("meter", "unlit")
    capColor: Tokens.color("meter", "cap")
}

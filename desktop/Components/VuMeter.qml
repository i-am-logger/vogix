// A VU channel: terse uppercase label, segmented meter with peak cap,
// fixed-width dB readout. Purely presentational — the caller feeds
// normalized value/peak (0..1 in the dB window) and the display dB.
// Default thresholds sit at −12/−6 dB of the 40dB window (0.7/0.85).
import QtQuick
import qs.Vogix

Grid {
    id: root

    property string label: ""
    property real value: 0        // 0..1 within the dB window
    property real peak: -1        // 0..1 cap position, -1 = none
    property real db: NaN         // display dB; non-finite renders as ---
    property bool vertical: false
    property real meterLength: Metrics.body * 6
    property real meterThickness: Math.round(Metrics.body * 0.625)

    property real warnAt: 0.7
    property real dangerAt: 0.85

    property color low: "#ff00ff"
    property color mid: "#ff00ff"
    property color high: "#ff00ff"
    property color unlit: "#ff00ff"
    property color capColor: "#ff00ff"
    property color labelColor: "#ff00ff"
    property color valueColor: "#ff00ff"

    columns: vertical ? 1 : 3
    spacing: Metrics.unit
    verticalItemAlignment: Grid.AlignVCenter
    horizontalItemAlignment: Grid.AlignHCenter

    HudLabel {
        text: root.label
        color: root.labelColor
        visible: root.label !== ""
    }

    SegmentedMeter {
        width: root.vertical ? root.meterThickness : root.meterLength
        height: root.vertical ? root.meterLength : root.meterThickness
        vertical: root.vertical
        value: root.value
        peak: root.peak
        warnAt: root.warnAt
        dangerAt: root.dangerAt
        low: root.low
        mid: root.mid
        high: root.high
        unlit: root.unlit
        capColor: root.capColor
    }

    NumericText {
        text: isFinite(root.db) ? root.db.toFixed(1) : "---"
        widestText: "-40.0"
        font.pixelSize: Metrics.caption
        color: root.valueColor
    }
}

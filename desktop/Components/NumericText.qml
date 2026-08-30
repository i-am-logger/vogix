// A numeric readout that NEVER jitters: width is reserved for the widest
// string the value can take (TextMetrics), so 9% → 100% doesn't reflow
// the bar. Right-aligned inside the reservation, monospace does the rest.
import QtQuick
import qs.Vogix

Text {
    id: root

    // The widest value this readout can show, e.g. "100%", "-60.0", "999.9M".
    property string widestText: "100%"

    font.family: Config.fontFamily
    font.pixelSize: Metrics.body
    horizontalAlignment: Text.AlignRight
    verticalAlignment: Text.AlignVCenter

    TextMetrics {
        id: metrics
        font: root.font
        text: root.widestText
    }

    width: Math.ceil(Math.max(implicitWidth, metrics.width))
}

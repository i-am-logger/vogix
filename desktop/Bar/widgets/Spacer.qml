// A fixed gap along the bar's axis — layout punctuation, renders nothing.
import QtQuick
import qs.Bar.widgets
import qs.Vogix

Item {
    property BarAxis axis: null

    implicitWidth: (axis?.vertical ?? false) ? 1 : Metrics.body * 1.5
    implicitHeight: (axis?.vertical ?? false) ? Metrics.body * 1.5 : 1
}

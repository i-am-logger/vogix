// The context a bar hands its widgets: Section injects it post-load into
// any widget declaring `property BarAxis axis`. Widgets that ignore it
// (single glyphs) simply don't declare the property.
import QtQuick

QtObject {
    property bool vertical: false
    property int thickness: 32
    property string edge: "top"
    // The bar is on screen (BarState.live): widgets hold their data
    // sources through a Lease on this, so a hidden bar samples nothing.
    property bool live: false
}

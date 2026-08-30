// The orientation context a bar hands its widgets: Section injects it
// post-load into any widget declaring `property QtObject axis`. Widgets
// that ignore it (single glyphs) simply don't declare the property.
import QtQuick

QtObject {
    property bool vertical: false
    property int thickness: 32
    property string edge: "top"
}

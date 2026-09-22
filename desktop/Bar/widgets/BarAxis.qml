// The context a bar hands its widgets: Section injects it post-load into
// any widget declaring `property BarAxis axis`. Widgets that ignore it
// (single glyphs) simply don't declare the property. It also knows the
// bar's window, so a widget can open its panel beside the bar it sits on.
import QtQuick
import Quickshell
import qs.Geometry
import qs.Services

QtObject {
    id: root

    property bool vertical: false
    property int thickness: 32
    property string edge: "top"
    // The bar is on screen (BarState.live): widgets hold their data
    // sources through a Lease on this, so a hidden bar samples nothing.
    property bool live: false
    property PanelWindow window: null

    // Where `item` (a widget on this bar) lies on the bar's screen.
    function screenRect(item: Item): rect {
        if (!root.window)
            return Qt.rect(0, 0, 0, 0);
        const screen = root.window.screen;
        const origin = Placement.barOrigin(root.edge, root.thickness,
            Qt.size(screen?.width ?? 0, screen?.height ?? 0), BarState.thickness("top"));
        const r = root.window.itemRect(item);
        return Qt.rect(origin.x + r.x, origin.y + r.y, r.width, r.height);
    }

    // Toggle panel `name`, opening it beside this bar, centred on `item`.
    function togglePanel(name: string, item: Item): string {
        return Panels.toggleAt(name, root.edge, root.window?.screen ?? null, root.screenRect(item));
    }
}

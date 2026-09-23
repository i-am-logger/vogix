pragma ComponentBehavior: Bound
// One bar section: the widgets desktop.json names, in order, laid along
// the bar's axis. Names resolve through the widget registry
// (WidgetRegistry, widgets/registry.json); `custom/<name>` places a cell
// desktop.json defines. A name the registry lacks, a widget on a bar of
// the orientation the registry keeps it off, or an undefined custom cell
// renders LOUD — a magenta tile, and a warning naming why. The config is
// Nix-generated and both the options and `vogix desktop check` reject
// such placements, so seeing one is a generator bug. The axis context is
// injected post-load into any widget that declares `property BarAxis
// axis`.
import QtQuick
import QtQuick.Layouts
import qs.Bar.widgets
import qs.Services
import qs.Vogix

GridLayout {
    id: root

    property list<string> names
    property BarAxis axis: null

    readonly property bool vertical: axis?.vertical ?? false

    // { source, problem }: the component that renders `name` here, and
    // why it cannot render ("" when it can).
    function resolve(name: string): var {
        const unknown = problem => ({ source: "widgets/Unknown.qml", problem: problem });
        if (name.startsWith("custom/"))
            return (Config.doc.custom ?? {})[name.slice(7)] !== undefined
                ? { source: "widgets/CustomCell.qml", problem: "" }
                : unknown("desktop.json defines no such custom cell");
        const entry = WidgetRegistry.widgets[name];
        if (entry === undefined)
            return unknown("not in the widget registry");
        const orientation = root.vertical ? "vertical" : "horizontal";
        if ((entry.orientation ?? orientation) !== orientation)
            return unknown(entry.orientation + "-only, placed on a " + orientation + " bar");
        return { source: "widgets/" + entry.component + ".qml", problem: "" };
    }

    flow: vertical ? GridLayout.TopToBottom : GridLayout.LeftToRight
    rowSpacing: Metrics.unit * 3
    columnSpacing: Metrics.unit * 3

    Repeater {
        model: root.names

        Loader {
            id: slot

            required property string modelData
            readonly property var resolved: root.resolve(slot.modelData)
            // The bar this slot sits on, for BarState's placement listing.
            readonly property BarAxis axis: root.axis

            Component.onCompleted: BarState.place(slot)
            Component.onDestruction: BarState.unplace(slot)

            Layout.alignment: root.vertical ? Qt.AlignHCenter : Qt.AlignVCenter
            source: slot.resolved.source
            onLoaded: {
                if (slot.resolved.problem !== "")
                    console.warn("vogix: bar widget '" + slot.modelData + "' cannot render: " + slot.resolved.problem);
                // Duck-typed injection: bracket access, because the static
                // item type here is just Item.
                if ("widgetName" in item)
                    item["widgetName"] = slot.modelData;
                if ("axis" in item)
                    item["axis"] = root.axis;
            }
        }
    }
}

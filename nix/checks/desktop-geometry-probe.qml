pragma ComponentBehavior: Bound
// checks.desktop-smoke's geometry probe. The smoke stages it as shell.qml
// over a copy of the shipped QML tree, so `qs.` resolves to the shipped
// modules, and runs it with the shipped default desktop.json. The real
// Section loads the real widgets, backed by the real singletons, in real
// windows. Three checks, one verdict (the exit status):
//
// - every widget the registry (WidgetRegistry) names, on a horizontal bar
//   and, where it may sit there, on a vertical one: each Canvas it shows
//   lays out at a non-zero size, measured on the canvas itself (a
//   FrameCell keeps a minimum size of its own around empty content).
//   `GEOMETRY <edge> <widget> <w>x<h>` per canvas;
// - a frame of audio data resizes no canvas: the instruments keep their
//   footprint whether or not anything plays. `FOOTPRINT <edge> <widget>`
//   per canvas that moved;
// - the default layout fits its bars: every cell's extent across its bar
//   is within the bar's size. `FIT <edge> <widget> <w>x<h> in <size>`
//   per cell.
import QtQuick
import Quickshell
import qs.Bar.widgets
import qs.Services
import qs.Vogix
import "Bar"

ShellRoot {
    id: root

    readonly property list<string> names: Object.keys(WidgetRegistry.widgets)
    readonly property list<string> railNames:
        root.names.filter(n => WidgetRegistry.widgets[n].horizontalOnly !== true)
    readonly property int bottomSize: ((Config.bars ?? {}).bottom ?? {}).size ?? 96
    readonly property int rightSize: ((Config.bars ?? {}).right ?? {}).size ?? 114
    readonly property list<string> edges:
        Object.keys(Config.bars ?? {}).filter(e => Config.bars[e].enable === true)

    // The default layout's sections: { edge, size, vertical, section }.
    property var layoutSections: []

    function canvasesUnder(item: Item): list<Item> {
        let found = [];
        for (let i = 0; i < item.children.length; i++) {
            const child = item.children[i];
            if (child instanceof Canvas)
                found.push(child);
            found = found.concat(root.canvasesUnder(child));
        }
        return found;
    }

    // The canvases a widget shows, itself included (the spectrums are one).
    function shownCanvases(item: Item): list<Item> {
        const own = item instanceof Canvas ? [item] : [];
        return own.concat(root.canvasesUnder(item)).filter(c => c.visible);
    }

    // [{ name, item }] for every widget a section loaded; a name that did
    // not load is logged and counted.
    function loaded(edge: string, section: Item): var {
        const out = [];
        for (let i = 0; i < section.children.length; i++) {
            const child = section.children[i];
            // Bracket access: modelData is the Section delegate's own
            // required property, not a Loader member.
            if (!(child instanceof Loader))
                continue;
            if (child.status !== Loader.Ready) {
                console.error("GEOMETRY", edge, child["modelData"], "did not load");
                root.failures++;
                continue;
            }
            out.push({ name: child["modelData"], item: child.item });
        }
        return out;
    }

    property int failures: 0
    // edge/widget/index → "WxH", taken before the audio frame.
    property var before: ({})

    function measureCanvases(edge: string, section: Item, record: bool): void {
        for (const w of root.loaded(edge, section)) {
            const canvases = root.shownCanvases(w.item);
            for (let i = 0; i < canvases.length; i++) {
                const size = `${canvases[i].width}x${canvases[i].height}`;
                const key = `${edge} ${w.name} ${i}`;
                if (record) {
                    console.info("GEOMETRY", edge, w.name, size);
                    if (!(canvases[i].width >= 1 && canvases[i].height >= 1))
                        root.failures++;
                    root.before[key] = size;
                } else if (root.before[key] !== size) {
                    console.error("FOOTPRINT", edge, w.name, root.before[key], "->", size);
                    root.failures++;
                }
            }
        }
    }

    function measureFit(): void {
        for (const s of root.layoutSections) {
            for (const w of root.loaded(s.edge, s.section)) {
                if (!w.item.visible)
                    continue;
                const across = s.vertical ? w.item.width : w.item.height;
                const line = `${w.item.width}x${w.item.height} in ${s.size}`;
                if (across > s.size) {
                    console.error("FIT", s.edge, w.name, line, "overflows its bar");
                    root.failures++;
                } else {
                    console.info("FIT", s.edge, w.name, line);
                }
            }
        }
    }

    // Every registry widget, sized like the default bottom bar and right
    // rail.
    FloatingWindow {
        implicitWidth: 1920
        implicitHeight: root.bottomSize

        Section {
            id: horizontal

            names: root.names
            axis: BarAxis {
                edge: "bottom"
                thickness: root.bottomSize
            }
        }
    }

    FloatingWindow {
        implicitWidth: root.rightSize
        implicitHeight: 1080

        Section {
            id: vertical

            names: root.railNames
            axis: BarAxis {
                edge: "right"
                thickness: root.rightSize
                vertical: true
            }
        }
    }

    // The default layout, one window per enabled bar.
    Variants {
        model: root.edges

        FloatingWindow {
            id: barWindow

            required property string modelData
            readonly property var bar: Config.bars[barWindow.modelData]
            readonly property bool vertical: barWindow.modelData === "left" || barWindow.modelData === "right"

            implicitWidth: barWindow.vertical ? barWindow.bar.size : 1920
            implicitHeight: barWindow.vertical ? 1080 : barWindow.bar.size

            Column {
                Repeater {
                    model: ["start", "center", "end"]

                    Section {
                        id: layoutSection

                        required property string modelData

                        names: barWindow.bar.layout[layoutSection.modelData] ?? []
                        axis: BarAxis {
                            edge: barWindow.modelData
                            thickness: barWindow.bar.size
                            vertical: barWindow.vertical
                        }
                        Component.onCompleted: root.layoutSections = root.layoutSections.concat([{
                            edge: barWindow.modelData,
                            size: barWindow.bar.size,
                            vertical: barWindow.vertical,
                            section: layoutSection
                        }])
                    }
                }
            }
        }
    }

    // Measured once the windows have laid out and drawn (a layout-sized
    // canvas has no size before the first polish), then again after one
    // frame of audio reaches the spectrum.
    Timer {
        id: first

        interval: 1500
        running: true
        onTriggered: {
            root.measureCanvases("bottom", horizontal, true);
            root.measureCanvases("right", vertical, true);
            root.measureFit();
            Cava.values = Array(Cava.bars * 2).fill(0.5);
            second.start();
        }
    }

    Timer {
        id: second

        interval: 500
        onTriggered: {
            root.measureCanvases("bottom", horizontal, false);
            root.measureCanvases("right", vertical, false);
            Qt.exit(root.failures === 0 ? 0 : 1);
        }
    }
}

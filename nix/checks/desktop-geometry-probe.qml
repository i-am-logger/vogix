pragma ComponentBehavior: Bound
// checks.desktop-smoke's geometry probe. The smoke stages it as shell.qml
// over a copy of the shipped QML tree, so `qs.` resolves to the shipped
// modules, and runs it with the shipped default desktop.json. The real
// Section loads the real widgets, backed by the real singletons, in real
// windows: one per bar edge, at that bar's size, holding every widget the
// registry (WidgetRegistry) lets that edge place. Three checks, one
// verdict (the exit status):
//
// - each Canvas a widget shows lays out at a non-zero size, measured on
//   the canvas itself (a FrameCell keeps a minimum size of its own
//   around empty content). `GEOMETRY <edge> <widget> <w>x<h>` per
//   canvas;
// - a frame of audio data resizes no canvas: the instruments keep their
//   footprint whether or not anything plays. `FOOTPRINT <edge> <widget>`
//   per canvas that moved;
// - every placement fits its bar: a widget's extent across the bar is
//   within the bar's size, so a widget the registry does not confine to
//   one orientation fits both. `FIT <edge> <widget> <w>x<h> in <size>`
//   per widget.
import QtQuick
import Quickshell
import qs.Bar.widgets
import qs.Services
import qs.Vogix
import "Bar"

ShellRoot {
    id: root

    readonly property list<string> names: Object.keys(WidgetRegistry.widgets)
    readonly property list<string> edges: Object.keys(Config.bars ?? {})

    // The registry names a bar of this orientation may place.
    function placeable(vertical: bool): list<string> {
        const orientation = vertical ? "vertical" : "horizontal";
        return root.names.filter(n => (WidgetRegistry.widgets[n].orientation ?? orientation) === orientation);
    }

    // Every edge's section: { edge, size, vertical, section }.
    property var sections: []

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

    function measureCanvases(s: var, record: bool): void {
        for (const w of root.loaded(s.edge, s.section)) {
            const canvases = root.shownCanvases(w.item);
            for (let i = 0; i < canvases.length; i++) {
                const size = `${canvases[i].width}x${canvases[i].height}`;
                const key = `${s.edge} ${w.name} ${i}`;
                if (record) {
                    console.info("GEOMETRY", s.edge, w.name, size);
                    if (!(canvases[i].width >= 1 && canvases[i].height >= 1))
                        root.failures++;
                    root.before[key] = size;
                } else if (root.before[key] !== size) {
                    console.error("FOOTPRINT", s.edge, w.name, root.before[key], "->", size);
                    root.failures++;
                }
            }
        }
    }

    function measureFit(s: var): void {
        for (const w of root.loaded(s.edge, s.section)) {
            if (!w.item.visible) {
                console.info("FIT", s.edge, w.name, "hidden here, not measured");
                continue;
            }
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

    Variants {
        model: root.edges

        FloatingWindow {
            id: barWindow

            required property string modelData
            readonly property bool vertical: barWindow.modelData === "left" || barWindow.modelData === "right"
            readonly property int size: Config.bars[barWindow.modelData].size

            implicitWidth: barWindow.vertical ? barWindow.size : 1920
            implicitHeight: barWindow.vertical ? 1080 : barWindow.size

            Section {
                id: section

                names: root.placeable(barWindow.vertical)
                axis: BarAxis {
                    edge: barWindow.modelData
                    thickness: barWindow.size
                    vertical: barWindow.vertical
                }
                Component.onCompleted: root.sections = root.sections.concat([{
                    edge: barWindow.modelData,
                    size: barWindow.size,
                    vertical: barWindow.vertical,
                    section: section
                }])
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
            if (root.sections.length !== 4) {
                console.error("GEOMETRY", root.sections.length, "of 4 bar edges laid out");
                root.failures++;
            }
            for (const s of root.sections) {
                root.measureCanvases(s, true);
                root.measureFit(s);
            }
            Cava.values = Array(Cava.bars * 2).fill(0.5);
            second.start();
        }
    }

    Timer {
        id: second

        interval: 500
        onTriggered: {
            for (const s of root.sections)
                root.measureCanvases(s, false);
            Qt.exit(root.failures === 0 ? 0 : 1);
        }
    }
}

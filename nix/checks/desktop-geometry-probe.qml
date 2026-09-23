pragma ComponentBehavior: Bound
// checks.desktop-smoke's geometry probe. The smoke stages it as shell.qml
// over a copy of the shipped QML tree, so `qs.` resolves to the shipped
// modules, and runs it with the shipped default desktop.json and every
// data source fed (desktop-geometry-feed.nix). The real Section loads the
// real widgets, backed by the real singletons, in real windows: one per
// bar edge, at that bar's size, holding every widget the registry
// (WidgetRegistry) lets that edge place. Three checks, one verdict (the
// exit status):
//
// - each Canvas a widget shows lays out at a non-zero size, measured on
//   the canvas itself (a FrameCell keeps a minimum size of its own
//   around empty content). `GEOMETRY <edge> <widget> <w>x<h>` per
//   canvas;
// - a frame of audio data resizes no canvas: the instruments keep their
//   footprint whether or not anything plays. `FOOTPRINT <edge> <widget>`
//   per canvas that moved;
// - every placement shows and fits its bar: a widget's extent across the
//   bar is within the bar's size, so a widget the registry does not
//   confine to one orientation fits both. `FIT <edge> <widget> <w>x<h> in
//   <size>` per widget.
import QtQuick
import Quickshell
import Quickshell.Services.UPower
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
    // not load is logged and counted once.
    function loaded(edge: string, section: Item, report: bool): var {
        const out = [];
        for (let i = 0; i < section.children.length; i++) {
            const child = section.children[i];
            // Bracket access: modelData is the Section delegate's own
            // required property, not a Loader member.
            if (!(child instanceof Loader))
                continue;
            if (child.status !== Loader.Ready) {
                if (report) {
                    console.error("GEOMETRY", edge, child["modelData"], "did not load");
                    root.failures++;
                }
                continue;
            }
            out.push({ name: child["modelData"], item: child.item });
        }
        return out;
    }

    // "<edge> <widget>" for every loaded widget not showing.
    function hidden(): list<string> {
        const out = [];
        for (const s of root.sections)
            for (const w of root.loaded(s.edge, s.section, false))
                if (!w.item.visible)
                    out.push(`${s.edge} ${w.name}`);
        return out;
    }

    // The widgets whose content arrives in parts have all of it: every
    // UPower device's properties (one row per battery), and both privacy
    // flags (a glyph each), which come from two sources.
    function complete(): bool {
        return UPower.displayDevice.ready && [...UPower.devices.values].every(d => d.ready)
            && Privacy.micInUse && Privacy.screencast;
    }

    // Lays every item under `item` out now, innermost first, rather than
    // on the next frame: a positioner or a layout computes its size in its
    // polish.
    function polishAll(item: Item): void {
        for (let i = 0; i < item.children.length; i++)
            root.polishAll(item.children[i]);
        item.ensurePolished();
    }

    property int failures: 0
    // edge/widget/index → "WxH", taken before the audio frame.
    property var before: ({})

    function measureCanvases(s: var, record: bool): void {
        for (const w of root.loaded(s.edge, s.section, record)) {
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
        for (const w of root.loaded(s.edge, s.section, false)) {
            if (!w.item.visible) {
                console.error("FIT", s.edge, w.name, "never showed: its data did not arrive");
                root.failures++;
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

    // The probe's bars are not live, so their widgets start no samplers.
    // The swap cell shows only once the memory sampler has read
    // /proc/meminfo, so the probe holds that one, as a shown bar would.
    Component.onCompleted: SysStat.acquire(["memory"])

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

    // Measured once all four edges are laid out and every widget shows
    // the data it is fed, checked every 100 ms. After giveUpMs the probe
    // measures what shows and fails on each widget still hidden.
    readonly property int giveUpMs: 30000
    readonly property real startedAt: Date.now()

    Timer {
        id: ready

        interval: 100
        repeat: true
        running: true
        onTriggered: {
            const waited = Date.now() - root.startedAt;
            const shown = root.sections.length === 4 && root.hidden().length === 0 && root.complete();
            if (!shown && waited < root.giveUpMs)
                return;
            ready.stop();
            console.info("GEOMETRY", shown ? "every widget showing after" : "gave up after", waited, "ms");
            if (root.sections.length !== 4) {
                console.error("GEOMETRY", root.sections.length, "of 4 bar edges laid out");
                root.failures++;
            }
            if (!root.complete()) {
                console.error("GEOMETRY UPower or privacy data incomplete after", waited, "ms");
                root.failures++;
            }
            for (const s of root.sections)
                root.polishAll(s.section);
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
            for (const s of root.sections) {
                root.polishAll(s.section);
                root.measureCanvases(s, false);
            }
            Qt.exit(root.failures === 0 ? 0 : 1);
        }
    }
}

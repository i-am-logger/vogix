pragma ComponentBehavior: Bound
// checks.desktop-smoke's geometry probe. The smoke stages it as shell.qml
// over a copy of the shipped QML tree, so the real registry
// (Bar/Section.qml) loads the real widgets, backed by the real singletons,
// inside a real window. Every Canvas a probed widget draws on must lay out
// at a non-zero size: one `GEOMETRY <widget> <w>x<h>` line per canvas, and
// the exit status is the verdict.
import QtQuick
import Quickshell
import qs.Bar.widgets
import "Bar"

ShellRoot {
    id: root

    // Widgets whose instrument is a Canvas the widget sizes itself.
    readonly property list<string> probed: ["oscilloscope"]

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

    function loaderFor(name: string): Loader {
        for (let i = 0; i < section.children.length; i++) {
            const child = section.children[i];
            // Bracket access: modelData is the Section delegate's own
            // required property, not a Loader member.
            if (child instanceof Loader && child["modelData"] === name)
                return child;
        }
        return null;
    }

    function verdict(): int {
        let failures = 0;
        for (const name of root.probed) {
            const loader = root.loaderFor(name);
            if (loader === null || loader.status !== Loader.Ready) {
                console.error("GEOMETRY", name, "did not load");
                failures++;
                continue;
            }
            const canvases = root.canvasesUnder(loader.item);
            if (canvases.length === 0) {
                console.error("GEOMETRY", name, "draws on no canvas");
                failures++;
            }
            for (const canvas of canvases) {
                console.info("GEOMETRY", name, `${canvas.width}x${canvas.height}`);
                if (!(canvas.width >= 1 && canvas.height >= 1))
                    failures++;
            }
        }
        return failures === 0 ? 0 : 1;
    }

    // Sized like the default bottom bar, the edge the scope ships on.
    FloatingWindow {
        implicitWidth: 1280
        implicitHeight: 96

        Section {
            id: section

            names: root.probed
            axis: BarAxis {
                edge: "bottom"
                thickness: 96
            }
        }
    }

    Component.onCompleted: Qt.callLater(() => Qt.exit(root.verdict()))
}

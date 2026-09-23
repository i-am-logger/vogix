pragma ComponentBehavior: Bound
// checks.desktop-smoke's state probe. Staged as shell.qml over a copy of
// the shipped QML tree (like the geometry probe), it loads the real
// `window` and `mode` widgets through the real Section and the real
// notification popups, and follows what they show while their inputs
// change. Each check prints `STATE <check> <value>` once it holds; the
// probe exits 0 once all have. There is no deadline: the smoke's overall
// timeout is the only bound, and what each open check sees is logged
// each time that changes (`STATE waiting for <check>: …`).
//
// - window-title: the title comes from `hyprctl -j activewindow` (the
//   smoke's hyprctl fixture). Hyprland.activeToplevel stays null without
//   the toplevel-mapping protocol, as it does here.
// - mode-label: the mode cell shows input.json's label for the current
//   mode (the smoke writes both files).
// - mode-label-after-failure: once input.json is gone and the cell's
//   view reloads, the failed load clears the table, so the label falls
//   back to the bare mode name instead of the stale mapping.
// - card-scanlines: with background.scanlines on (the smoke runs this
//   probe with it), every notification card restored from the previous
//   runs carries the live scanline texture.
import QtQuick
import Quickshell
import Quickshell.Io
import qs.Bar.widgets
import qs.Components
import qs.Vogix
import "Bar"
import "Notifications"

ShellRoot {
    id: root

    // What the open checks saw when last logged.
    property string waitingFor: ""
    // check → the value it passed with; a check absent here has not.
    property var passed: ({})
    readonly property list<string> checks:
        ["window-title", "mode-label", "mode-label-after-failure", "card-scanlines"]
    // The mode checks run in order: the table must show before its file
    // goes away.
    property int modeStage: 0

    function widget(name: string): Item {
        for (let i = 0; i < section.children.length; i++) {
            const child = section.children[i];
            if (child instanceof Loader && child["modelData"] === name)
                return child.item;
        }
        return null;
    }

    // The FileView an item reads through, found by the path it names
    // (a FrameCell keeps its content, non-visual objects included, in an
    // inner slot).
    function viewUnder(item: Item, suffix: string): FileView {
        for (const kid of item.resources) {
            if (kid instanceof FileView && String(kid.path).endsWith(suffix))
                return kid;
        }
        for (const kid of item.children) {
            const found = root.viewUnder(kid, suffix);
            if (found !== null)
                return found;
        }
        return null;
    }

    // The scanline overlays under an item that are live.
    function liveOverlays(item: Item): int {
        let n = item instanceof ScanlineOverlay && item.active && item.status === Loader.Ready ? 1 : 0;
        for (const kid of item.children)
            n += root.liveOverlays(kid);
        return n;
    }

    function pass(check: string, value: string): void {
        if (root.passed[check] !== undefined)
            return;
        console.info("STATE", check, value);
        const next = Object.assign({}, root.passed);
        next[check] = value;
        root.passed = next;
    }

    function step(): void {
        const title = root.widget("window");
        const mode = root.widget("mode");
        const cards = popups.visiblePopups.length;
        const lit = root.liveOverlays(popups.contentItem);

        if (title !== null && title.text === "smoke window title")
            root.pass("window-title", title.text);

        if (root.modeStage === 0 && mode !== null && mode.modeLabel === "NRM-SMOKE") {
            root.pass("mode-label", mode.modeLabel);
            root.modeStage = 1;
            remove.running = true;
        } else if (root.modeStage === 2 && mode.modeLabel === Mode.mode) {
            root.pass("mode-label-after-failure", mode.modeLabel);
        }

        if (cards >= 2 && lit === cards)
            root.pass("card-scanlines", lit + "/" + cards);

        const open = root.checks.filter(c => root.passed[c] === undefined);
        if (open.length === 0) {
            Qt.exit(0);
        } else {
            const seen = {
                "window-title": "title '" + (title?.text ?? "") + "'",
                "mode-label": "label '" + (mode?.modeLabel ?? "") + "'",
                "mode-label-after-failure": "label '" + (mode?.modeLabel ?? "") + "', mode '" + Mode.mode + "'",
                "card-scanlines": lit + " live scanline overlays on " + cards + " cards"
            };
            const waiting = open.map(c => c + ": " + seen[c]).join("; ");
            if (waiting !== root.waitingFor) {
                root.waitingFor = waiting;
                console.info("STATE waiting for", waiting);
            }
        }
    }

    Process {
        id: remove

        command: ["rm", Paths.stateRoot + "/input.json"]
        onRunningChanged: {
            if (running)
                return;
            const view = root.viewUnder(root.widget("mode"), "/input.json");
            if (view === null) {
                console.error("STATE-FAIL mode-label-after-failure the mode cell reads no input.json view");
                Qt.exit(1);
                return;
            }
            view.reload();
            root.modeStage = 2;
        }
    }

    // The notification cards, restored from the state file.
    Popups {
        id: popups
    }

    FloatingWindow {
        implicitWidth: 1280
        implicitHeight: 96

        Section {
            id: section

            names: ["window", "mode"]
            axis: BarAxis {
                edge: "top"
                thickness: 96
            }
        }
    }

    Timer {
        interval: 100
        running: true
        repeat: true
        onTriggered: root.step()
    }
}

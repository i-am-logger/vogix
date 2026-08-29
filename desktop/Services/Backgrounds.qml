pragma Singleton
// The theme's background set (backgrounds.json, read through the
// current-theme chain so a theme switch flips it atomically with the
// palette) plus the per-user override and cycle position. Reloaded by the
// same `vogix desktop reload` verb as the rest of the contract.
import QtQuick
import Quickshell
import Quickshell.Io
import qs.Vogix

Singleton {
    id: root

    property var entries: []
    property int index: 0
    property string overridePath: ""

    // What the wallpaper layer should show right now.
    readonly property var current: {
        if (overridePath !== "")
            return { kind: "image", path: overridePath, name: "override" };
        if (entries.length === 0)
            return null;
        return entries[Math.min(index, entries.length - 1)];
    }

    function reload(): void {
        view.reload();
        overrideFile.reload();
    }

    function next(): string {
        if (root.entries.length === 0)
            return "no backgrounds";
        root.overridePath = "";
        root.index = (root.index + 1) % root.entries.length;
        saveState();
        return root.entries[root.index].name;
    }

    function set(path: string): string {
        root.overridePath = path;
        saveState();
        return "override: " + path;
    }

    function clear(): string {
        root.overridePath = "";
        root.index = 0;
        saveState();
        return root.current ? root.current.name : "none";
    }

    function status(): string {
        const c = root.current;
        return c ? (c.kind + " " + (c.path ?? "")) : "none";
    }

    function saveState(): void {
        overrideFile.setText(JSON.stringify({
            override: root.overridePath,
            index: root.index
        }));
    }

    FileView {
        id: view
        path: Paths.stateRoot + "/current-theme/vogix-desktop/backgrounds.json"
        watchChanges: false
        onLoaded: {
            try {
                root.entries = JSON.parse(text()).backgrounds ?? [];
            } catch (e) {
                root.entries = [];
            }
        }
        onLoadFailed: root.entries = []
    }

    FileView {
        id: overrideFile
        path: Paths.stateRoot + "/desktop/background.json"
        watchChanges: false
        onLoaded: {
            try {
                const s = JSON.parse(text());
                root.overridePath = s.override ?? "";
                root.index = s.index ?? 0;
            } catch (e) {}
        }
    }
}

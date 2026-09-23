pragma Singleton
pragma ComponentBehavior: Bound
// The custom cells' commands (desktop.json `custom`): one runner per
// entry, shared by every cell that places it (`custom/<name>`) on every
// bar and screen. A runner knows how many of those cells exist (placed)
// and how many sit on a bar that is on screen (live); its command runs
// only while one is live.
import QtQuick
import Quickshell
import qs.Vogix

Singleton {
    id: root

    readonly property var defs: Config.doc.custom ?? ({})
    readonly property list<string> names: Object.keys(defs)

    // name → how many cells exist, and how many are live; each reassigned
    // whole so every runner's bindings re-evaluate.
    property var placedRefs: ({})
    property var liveRefs: ({})

    function _count(refs: var, name: string, delta: int): var {
        const next = Object.assign({}, refs);
        next[name] = Math.max(0, (next[name] ?? 0) + delta);
        return next;
    }

    function place(name: string): void {
        root.placedRefs = root._count(root.placedRefs, name, 1);
    }

    function unplace(name: string): void {
        root.placedRefs = root._count(root.placedRefs, name, -1);
    }

    function acquire(name: string): void {
        root.liveRefs = root._count(root.liveRefs, name, 1);
    }

    function release(name: string): void {
        root.liveRefs = root._count(root.liveRefs, name, -1);
    }

    function runner(name: string): var {
        const all = runners.instances;
        for (let i = 0; i < all.length; i++) {
            if (all[i].name === name)
                return all[i];
        }
        return null;
    }

    function click(name: string): void {
        runner(name)?.click();
    }

    // A refresh while no bar shows the cell is kept for when one does.
    function refresh(name: string): string {
        const r = runner(name);
        if (r === null)
            return "unknown custom cell: " + name;
        if (!r.placed)
            return "inactive: no bar shows custom/" + name;
        r.trigger();
        return r.active ? "refreshing" : "queued: custom/" + name + " runs once its bar is on screen";
    }

    function status(name: string): string {
        const r = runner(name);
        return r === null ? "unknown custom cell: " + name : r.status;
    }

    Variants {
        id: runners
        model: root.names

        CustomRunner {
            id: entry

            required property var modelData

            name: entry.modelData
            def: root.defs[entry.modelData] ?? ({})
            placed: (root.placedRefs[entry.modelData] ?? 0) > 0
            active: (root.liveRefs[entry.modelData] ?? 0) > 0
        }
    }
}

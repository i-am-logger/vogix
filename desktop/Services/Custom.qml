pragma Singleton
pragma ComponentBehavior: Bound
// The custom cells' commands (desktop.json `custom`): one runner per
// entry, shared by every cell that places it (`custom/<name>`) on every
// bar and screen, and live only while at least one of those cells exists.
import QtQuick
import Quickshell
import qs.Vogix

Singleton {
    id: root

    readonly property var defs: Config.doc.custom ?? ({})
    readonly property list<string> names: Object.keys(defs)

    // name → how many cells show it; reassigned whole so every runner's
    // `active` binding re-evaluates.
    property var refs: ({})

    function acquire(name: string): void {
        const next = Object.assign({}, root.refs);
        next[name] = (next[name] ?? 0) + 1;
        root.refs = next;
    }

    function release(name: string): void {
        const next = Object.assign({}, root.refs);
        next[name] = Math.max(0, (next[name] ?? 0) - 1);
        root.refs = next;
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

    function refresh(name: string): string {
        const r = runner(name);
        if (r === null)
            return "unknown custom cell: " + name;
        if (!r.active)
            return "inactive: no bar shows custom/" + name;
        r.trigger();
        return "refreshing";
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
            active: (root.refs[entry.modelData] ?? 0) > 0
        }
    }
}

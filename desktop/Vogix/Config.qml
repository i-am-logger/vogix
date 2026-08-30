// The desktop.json reader (per-user shell configuration, Nix-generated).
// watchChanges is OFF: home-manager swaps the store symlink with ln -sfn,
// invisible to the watcher — the unit's X-Reload-Triggers + ExecReload
// deliver config changes as the same IPC reload the theme uses.
pragma Singleton
import QtQuick
import Quickshell
import Quickshell.Io

Singleton {
    id: root

    property var doc: ({})
    readonly property var bar: doc.bar ?? ({})
    // The canonical four-edge table (schema 2). A schema-1 doc carries only
    // `bar`; synthesize its edge and leave the other three off, so a shell
    // ahead of its config still renders the bar the user configured.
    readonly property var bars: doc.bars ?? legacyBars()
    readonly property string fontFamily: (doc.font ?? {}).family ?? "monospace"
    readonly property int fontSize: (doc.font ?? {}).size ?? 16

    function legacyBars(): var {
        const b = doc.bar ?? {};
        const l = b.layout ?? {};
        const off = { enable: false, size: 0, layout: { start: [], center: [], end: [] } };
        const on = {
            enable: b.enable ?? false,
            size: b.height ?? 32,
            layout: { start: l.left ?? [], center: l.center ?? [], end: l.right ?? [] },
        };
        const edge = b.position ?? "top";
        return {
            top: edge === "top" ? on : off,
            bottom: edge === "bottom" ? on : off,
            left: off,
            right: off,
        };
    }

    function reload(): void {
        view.reload();
    }

    FileView {
        id: view
        path: Paths.stateRoot + "/desktop.json"
        watchChanges: false
        onLoaded: root.doc = JSON.parse(text())
        onLoadFailed: console.warn("vogix: cannot read", path)
    }
}

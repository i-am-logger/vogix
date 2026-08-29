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
    readonly property string fontFamily: (doc.font ?? {}).family ?? "monospace"
    readonly property int fontSize: (doc.font ?? {}).size ?? 13

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

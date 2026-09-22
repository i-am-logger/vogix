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

    // The desktop.json schema this shell reads. A document of any other
    // schema was written by a different vogix than the one running: it is
    // refused LOUDLY and its bars stay off until a rebuild regenerates it.
    readonly property int schema: 2

    property var doc: ({})
    readonly property bool supported: (doc.schema ?? 0) === schema
    readonly property var bars: supported ? (doc.bars ?? ({})) : ({})
    readonly property string fontFamily: (doc.font ?? {}).family ?? "monospace"
    readonly property int fontSize: (doc.font ?? {}).size ?? 16

    function reload(): void {
        view.reload();
    }

    FileView {
        id: view
        path: Paths.stateRoot + "/desktop.json"
        watchChanges: false
        onLoaded: {
            const parsed = JSON.parse(text());
            if ((parsed.schema ?? 0) !== root.schema)
                console.warn("vogix: desktop.json schema " + parsed.schema
                    + " is not supported (this shell reads schema " + root.schema
                    + "); rebuild to regenerate it");
            root.doc = parsed;
        }
        onLoadFailed: console.warn("vogix: cannot read", path)
    }
}

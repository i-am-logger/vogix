// The theme.json contract reader. watchChanges is OFF on purpose: a theme
// switch swaps a store symlink, which Qt's file watcher never sees — the
// reload arrives as the `vogix desktop reload` verb over IPC instead.
// Always route through reload() -> onLoaded (text() is stale inside
// change signals), and reassign the whole doc so bindings re-evaluate.
pragma Singleton
import QtQuick
import Quickshell
import Quickshell.Io

Singleton {
    id: root

    property var doc: ({})
    readonly property var semantic: doc.semantic ?? ({})
    readonly property var palette: doc.palette ?? ({})
    readonly property string name: doc.theme ?? "vogix"
    readonly property string variant: doc.variant ?? ""
    readonly property string polarity: doc.polarity ?? "dark"

    function reload(): void {
        view.reload();
    }

    FileView {
        id: view
        path: Paths.configRoot + "/vogix-desktop/theme.json"
        watchChanges: false
        onLoaded: root.doc = JSON.parse(text())
        onLoadFailed: console.warn("vogix: cannot read", path)
    }
}

// The engine's current input mode (~/.local/state/vogix/current-mode).
// This file IS written in place (std::fs::write), so watching works — the
// one contract file where it does. Missing file = the root mode.
pragma Singleton
import QtQuick
import Quickshell
import Quickshell.Io

Singleton {
    id: root

    property string mode: "app"

    FileView {
        path: Paths.stateRoot + "/current-mode"
        watchChanges: true
        onFileChanged: reload()
        onLoaded: {
            const t = text().trim();
            if (t.length > 0)
                root.mode = t;
        }
        onLoadFailed: root.mode = "app"
    }
}

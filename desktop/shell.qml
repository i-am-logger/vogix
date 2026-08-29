//@ pragma UseQApplication
// vogix desktop shell (v1, quickshell) — the rendering layer over the
// vogix contract: theme.json (colors), desktop.json (layout), current-mode
// (the engine's input mode). Everything here is replaceable by the v2 Rust
// vogix-desktop; the contract files, verbs and unit name are not.
import QtQuick
import Quickshell
import Quickshell.Io
import qs.Vogix
import "Bar"

ShellRoot {
    id: root

    Bar {
        id: bar
    }

    // IPC targets mirror the `vogix desktop …` verbs 1:1; nothing else may
    // talk to these directly (the verb is the contract, qs the transport).
    IpcHandler {
        target: "theme"

        function reload(): void {
            Theme.reload();
            Config.reload();
        }
    }

    IpcHandler {
        target: "bar"

        function show(): void { bar.hidden = false; }
        function hide(): void { bar.hidden = true; }
        function toggle(): void { bar.hidden = !bar.hidden; }
        function status(): string { return bar.hidden ? "hidden" : "shown"; }
    }
}

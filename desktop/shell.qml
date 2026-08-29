//@ pragma UseQApplication
// vogix desktop shell (v1, quickshell) — the rendering layer over the
// vogix contract: theme.json (colors), desktop.json (layout), current-mode
// (the engine's input mode). Everything here is replaceable by the v2 Rust
// vogix-desktop; the contract files, verbs and unit name are not.
import QtQuick
import Quickshell
import Quickshell.Io
import qs.Services
import qs.Vogix
import "Bar"
import "Notifications"
import "Osd"
import "Polkit"

ShellRoot {
    id: root

    Bar {
        id: bar
    }

    LazyLoader {
        active: (Config.doc.notifications ?? {}).enable ?? true
        component: Popups {}
    }

    LazyLoader {
        active: (Config.doc.osd ?? {}).enable ?? true
        component: OsdWindow {}
    }

    LazyLoader {
        active: (Config.doc.polkit ?? {}).enable ?? true
        component: PolkitDialog {}
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

    IpcHandler {
        target: "notify"

        function dismiss(): void { Notifs.dismissNewest(); }
        function dismissAll(): void { Notifs.dismissAll(); }
        function dndOn(): void { Notifs.setDnd(true); }
        function dndOff(): void { Notifs.setDnd(false); }
        function dndToggle(): string {
            Notifs.setDnd(!Notifs.dnd);
            return Notifs.dnd ? "on" : "off";
        }
        function dndStatus(): string { return Notifs.dnd ? "on" : "off"; }
        function count(): int { return Notifs.popups.length; }
    }

    IpcHandler {
        target: "osd"

        // value: 0..100 (percent), -1 = no gauge.
        function show(kind: string, value: int, muted: bool, message: string): void {
            Osd.show(kind, value >= 0 ? value / 100.0 : -1, muted, message);
        }
    }
}

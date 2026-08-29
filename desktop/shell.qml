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
import "Background"
import "Bar"
import "Idle"
import "Launcher"
import "Notifications"
import "Osd"
import "Polkit"
import "Power"

ShellRoot {
    id: root

    // QML singletons are LAZY — they instantiate on first reference. These
    // services must run from startup (the notification server registers on
    // the bus, the idle monitors arm, the lock preflights its PAM check), so
    // reference them eagerly here.
    readonly property list<QtObject> services: [Notifs, Audio, Osd, Idle, Lock, Backgrounds, Launcher, Power]

    Bar {
        id: bar
    }

    LazyLoader {
        active: true
        component: DimOverlay {}
    }

    LazyLoader {
        active: (Config.doc.background ?? {}).enable ?? true
        component: Wallpaper {}
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

    LazyLoader {
        active: (Config.doc.launcher ?? {}).enable ?? true
        component: LauncherWindow {}
    }

    LazyLoader {
        active: (Config.doc.power ?? {}).enable ?? true
        component: PowerMenu {}
    }

    // IPC targets mirror the `vogix desktop …` verbs 1:1; nothing else may
    // talk to these directly (the verb is the contract, qs the transport).
    IpcHandler {
        target: "theme"

        function reload(): void {
            Theme.reload();
            Config.reload();
            Backgrounds.reload();
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
        target: "lock"

        function lock(): string { return Lock.lock(); }
        function status(): string { return Lock.status(); }
    }

    IpcHandler {
        target: "background"

        function set(path: string): string { return Backgrounds.set(path); }
        function next(): string { return Backgrounds.next(); }
        function clear(): string { return Backgrounds.clear(); }
        function status(): string { return Backgrounds.status(); }
    }

    IpcHandler {
        target: "launcher"

        function open(mode: string, query: string): string { return Launcher.openMode(mode, query); }
        function menu(summon: string): string { return Launcher.openMenu(summon); }
        function select(id: string, prompt: string): string { return Launcher.openSelect(id, prompt, false); }
        function inputText(id: string, prompt: string): string { return Launcher.openSelect(id, prompt, true); }
        function close(): string { return Launcher.close(); }
        function toggle(): string { return Launcher.toggle(); }
        function status(): string { return Launcher.status(); }
    }

    IpcHandler {
        target: "power"

        function open(): string { return Power.show(); }
        function close(): string { return Power.close(); }
        function toggle(): string { return Power.toggle(); }
        function status(): string { return Power.status(); }
    }

    IpcHandler {
        target: "osd"

        // value: 0..100 (percent), -1 = no gauge.
        function show(kind: string, value: int, muted: bool, message: string): void {
            Osd.show(kind, value >= 0 ? value / 100.0 : -1, muted, message);
        }
    }
}

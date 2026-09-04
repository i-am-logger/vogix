pragma Singleton
// The keyboard layout indicator's source of truth. The device that
// matters is `vogix-input` — the input engine's uinput re-emit device;
// its xkb state is what applications actually see — with the compositor's
// main keyboard as the fallback when the engine is off. Seeded from
// `hyprctl -j devices`, kept live by Hyprland's `activelayout` raw event
// filtered to that device, switched with `hyprctl switchxkblayout`.
import QtQuick
import Quickshell
import Quickshell.Hyprland
import Quickshell.Io

Singleton {
    id: root

    property string device: ""
    property string layoutFull: ""
    // The configured layout codes, in order ("us", "il", …) — the LANG
    // cell shows all of them with the active one lit.
    property list<string> layouts: []
    // CapsLock, which matters here because Alt+CapsLock is the layout switch:
    // a latched caps and a switched layout look the same from the keyboard.
    property bool capsOn: false

    function codeLabel(code: string): string {
        const map = { us: "EN", gb: "EN", il: "HE" };
        return map[code] ?? code.toUpperCase();
    }

    readonly property string label: {
        const l = layoutFull;
        if (l.startsWith("English"))
            return "EN";
        if (l.startsWith("Hebrew"))
            return "HE";
        return l === "" ? "??" : l.slice(0, 2).toUpperCase();
    }

    function next(): void {
        if (device !== "")
            switchProc.running = true;
    }

    Process {
        id: switchProc
        command: ["hyprctl", "switchxkblayout", root.device, "next"]
    }

    Process {
        id: layoutsProc
        running: true
        command: ["hyprctl", "-j", "getoption", "input:kb_layout"]

        stdout: StdioCollector {
            onStreamFinished: {
                try {
                    const str = JSON.parse(text).str ?? "";
                    root.layouts = str.split(",").map(s => s.trim()).filter(s => s !== "");
                } catch (e) {
                    root.layouts = [];
                }
            }
        }
    }

    Process {
        id: seedProc
        running: true
        command: ["hyprctl", "-j", "devices"]

        stdout: StdioCollector {
            onStreamFinished: {
                let doc;
                try {
                    doc = JSON.parse(text);
                } catch (e) {
                    console.warn("vogix: cannot parse hyprctl devices:", e.message);
                    return;
                }
                const kbs = doc.keyboards ?? [];
                const kb = kbs.find(k => k.name === "vogix-input")
                    ?? kbs.find(k => k.main)
                    ?? kbs[0];
                if (!kb)
                    return;
                root.device = kb.name;
                root.layoutFull = kb.active_keymap ?? "";
            }
        }
    }

    Connections {
        target: Hyprland

        function onRawEvent(event: HyprlandEvent): void {
            if (event.name !== "activelayout")
                return;
            // "device,layout name" — the layout name may itself contain
            // commas, so split only on the first.
            const data = event.data;
            const cut = data.indexOf(",");
            if (cut < 0)
                return;
            const dev = data.slice(0, cut);
            if (dev === root.device)
                root.layoutFull = data.slice(cut + 1);
        }
    }

    // POLLED, unlike the layout above, because nothing pushes it: Hyprland
    // emits `activelayout` but has no caps event, and `hyprctl devices` does
    // not carry the state. The LED is the only thing that reflects it, and its
    // path is enumeration-dependent (input5, input35, … change across reboots
    // and replugs), so this globs rather than naming a device. Several
    // keyboards each carry their own LED, hence the OR: caps is on if any of
    // them says so.
    //
    // The right long-term source is the input engine, which reads evdev and
    // already knows the exact state -- this would become an event instead of a
    // poll the moment it publishes one.
    Timer {
        running: true
        repeat: true
        interval: 250
        triggeredOnStart: true
        onTriggered: capsProc.running = true
    }

    Process {
        id: capsProc
        command: ["sh", "-c", "grep -qs 1 /sys/class/leds/*::capslock/brightness && echo 1 || echo 0"]

        stdout: StdioCollector {
            onStreamFinished: root.capsOn = text.trim() === "1"
        }
    }
}

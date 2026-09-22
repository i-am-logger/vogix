pragma Singleton
// The keyboard layout indicator's source of truth. The device that
// matters is `vogix-input` — the input engine's uinput re-emit device;
// its xkb state is what applications actually see — with the compositor's
// main keyboard as the fallback when the engine is off. That keyboard's
// layout list and the index of its active layout come from
// `hyprctl -j devices`, re-read on Hyprland's `activelayout` raw event;
// switched with `hyprctl switchxkblayout`.
import QtQuick
import Quickshell
import Quickshell.Hyprland
import Quickshell.Io

Singleton {
    id: root

    property string device: ""
    // The device's configured layout codes in xkb group order ("us", "il",
    // …) — the LANG cell shows all of them with the active one lit.
    property list<string> layouts: []
    // Index into `layouts` of the active layout, as Hyprland reports it;
    // -1 while unknown.
    property int activeIndex: -1
    // CapsLock, which matters here because Alt+CapsLock is the layout switch:
    // a latched caps and a switched layout look the same from the keyboard.
    property bool capsOn: false

    // Display alias for a layout code; codes without one show uppercased.
    function codeLabel(code: string): string {
        const map = { us: "EN", gb: "EN", il: "HE" };
        return map[code] ?? code.toUpperCase();
    }

    function next(): void {
        if (device !== "")
            switchProc.running = true;
    }

    function status(): string {
        const known = root.activeIndex >= 0 && root.activeIndex < root.layouts.length;
        return "device:" + (root.device !== "" ? root.device : "-")
            + " layouts:" + (root.layouts.length > 0 ? root.layouts.join(",") : "-")
            + " active:" + (known ? root.layouts[root.activeIndex] : "-");
    }

    Process {
        id: switchProc
        command: ["hyprctl", "switchxkblayout", root.device, "next"]
    }

    // Setting `running` while a query is in flight re-runs it once that one
    // exits (Process keeps the requested state), so a burst of events costs
    // at most one extra query and never reads a pre-switch state last.
    Process {
        id: devicesProc
        running: true
        command: ["hyprctl", "-j", "devices"]

        stdout: StdioCollector {
            onStreamFinished: {
                let doc;
                try {
                    doc = JSON.parse(text);
                } catch (e) {
                    console.warn("vogix: cannot parse hyprctl devices:", e.message);
                    doc = {};
                }
                const kbs = doc.keyboards ?? [];
                const kb = kbs.find(k => k.name === "vogix-input")
                    ?? kbs.find(k => k.main)
                    ?? kbs[0];
                if (!kb) {
                    root.device = "";
                    root.layouts = [];
                    root.activeIndex = -1;
                    return;
                }
                // Split without dropping empty entries: positions must stay
                // aligned with xkb's group indices.
                const codes = (kb.layout ?? "").split(",").map(s => s.trim());
                root.device = kb.name;
                root.layouts = codes.length === 1 && codes[0] === "" ? [] : codes;
                root.activeIndex = Number.isInteger(kb.active_layout_index)
                    ? kb.active_layout_index : -1;
            }
        }
    }

    Connections {
        target: Hyprland

        function onRawEvent(event: HyprlandEvent): void {
            switch (event.name) {
            // Posted whenever Hyprland applies a keymap to a keyboard: a
            // layout switch, `vogix-input` appearing after the shell, and a
            // config reload that changes kb_layout (on the Lua config provider
            // that lands after `configreloaded`, so this is the event to
            // follow). It names the layout, not its index: re-read both.
            case "activelayout":
                devicesProc.running = true;
                break;
            }
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

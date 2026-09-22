pragma Singleton
// The keyboard layout indicator's source of truth. The device that
// matters is `vogix-input` — the input engine's uinput re-emit device;
// its xkb state is what applications actually see — with the compositor's
// main keyboard as the fallback when the engine is off. That keyboard's
// layout list and the index of its active layout come from
// `hyprctl -j devices`, re-read on Hyprland's `activelayout` raw event;
// switched with `hyprctl switchxkblayout`. CapsLock comes from the input
// engine's input-locks.json, watched for changes.
import QtQuick
import Quickshell
import Quickshell.Hyprland
import Quickshell.Io
import qs.Vogix

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
    // Known only while the input engine publishes it (see the FileView).
    property bool capsKnown: false
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
            + " active:" + (known ? root.layouts[root.activeIndex] : "-")
            + " caps:" + (!root.capsKnown ? "unknown" : root.capsOn ? "on" : "off");
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

    // The input engine publishes the lock state its grabbed keyboards' LEDs
    // show, rewriting the document (tmp + rename) on each change and removing
    // it when the engine stops. `capsLock` is null when none of its keyboards
    // has a CapsLock LED; with no document or a null, caps is unknown.
    FileView {
        path: Paths.stateRoot + "/input-locks.json"
        watchChanges: true
        // Absent whenever the input engine is not running.
        printErrors: false
        onFileChanged: reload()
        onLoaded: {
            let caps = null;
            try {
                caps = JSON.parse(text()).capsLock ?? null;
            } catch (e) {
                console.warn("vogix: cannot parse input-locks.json:", e.message);
            }
            root.capsKnown = typeof caps === "boolean";
            root.capsOn = caps === true;
        }
        onLoadFailed: {
            root.capsKnown = false;
            root.capsOn = false;
        }
    }
}

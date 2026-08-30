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
}

pragma Singleton
// Which panel popup is open (one at a time): audio, network, bluetooth,
// power, monitor, tailscale, calendar, weather. Bar widgets toggle their
// panel; the `vogix desktop panel` verb mirrors it.
import QtQuick
import Quickshell

Singleton {
    id: root

    readonly property list<string> known: [
        "audio", "network", "bluetooth", "power",
        "monitor", "tailscale", "calendar", "weather", "agents",
    ]

    property string open: ""

    function show(name: string): string {
        if (!known.includes(name))
            return "unknown panel: " + name;
        root.open = name;
        return name;
    }

    function close(): string {
        root.open = "";
        return "closed";
    }

    function toggle(name: string): string {
        return root.open === name ? close() : show(name);
    }

    function status(): string {
        return root.open === "" ? "closed" : root.open;
    }
}

pragma Singleton
// Which panel popup is open (one at a time) and where it was summoned
// from. A bar widget opens its panel through its BarAxis (togglePanel →
// toggleAt), so the popup appears beside that widget's bar; the
// `vogix desktop panel` verb (show/toggle) has no widget and opens it
// under the top bar's end.
import QtQuick
import Quickshell

Singleton {
    id: root

    readonly property list<string> known: [
        "audio", "audio-out", "audio-in", "network", "bluetooth", "power",
        "monitor", "tailscale", "calendar", "weather", "agents",
    ]

    property string open: ""

    // The summoning widget: its bar's edge ("" = opened by the verb), that
    // bar's screen, and the widget's rect in that screen's coordinates.
    property string anchorEdge: ""
    property ShellScreen anchorScreen: null
    property rect anchorRect

    function show(name: string): string {
        return root.place(name, "", null, Qt.rect(0, 0, 0, 0));
    }

    function close(): string {
        root.open = "";
        root.anchorEdge = "";
        root.anchorScreen = null;
        return "closed";
    }

    function toggle(name: string): string {
        return root.open === name ? root.close() : root.show(name);
    }

    // A bar widget's toggle: `edge` and `screen` are its bar's, `at` the
    // widget's rect on that screen.
    function toggleAt(name: string, edge: string, screen: ShellScreen, at: rect): string {
        return root.open === name ? root.close() : root.place(name, edge, screen, at);
    }

    // The anchor lands before `open`, so the popup maps in place.
    function place(name: string, edge: string, screen: ShellScreen, at: rect): string {
        if (!root.known.includes(name))
            return "unknown panel: " + name;
        root.anchorEdge = edge;
        root.anchorScreen = screen;
        root.anchorRect = at;
        root.open = name;
        return name;
    }

    function status(): string {
        return root.open === "" ? "closed" : root.open;
    }
}

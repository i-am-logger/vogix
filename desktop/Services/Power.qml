pragma Singleton
// The power menu's state and actions. The overlay window renders `actions`;
// the same list backs the direct `vogix desktop power <action>` verbs, so
// menu and CLI cannot drift.
import QtQuick
import Quickshell
import qs.Services

Singleton {
    id: root

    property bool open: false
    property int cursor: 0

    readonly property var actions: [
        { id: "lock", icon: "󰌾", label: "Lock" },
        { id: "logout", icon: "󰍃", label: "Log out" },
        { id: "suspend", icon: "󰤄", label: "Suspend" },
        { id: "reboot", icon: "󰜉", label: "Reboot" },
        { id: "poweroff", icon: "󰐥", label: "Power off" },
    ]

    function show(): string {
        root.cursor = 0;
        root.open = true;
        return "open";
    }

    function close(): string {
        root.open = false;
        return "closed";
    }

    function toggle(): string {
        return root.open ? close() : show();
    }

    function status(): string {
        return root.open ? "open" : "closed";
    }

    function moveCursor(delta: int): void {
        root.cursor = Math.min(root.actions.length - 1,
                               Math.max(0, root.cursor + delta));
    }

    function run(id: string): string {
        root.open = false;
        switch (id) {
        case "lock":
            return Lock.lock();
        case "logout":
            // The compositor-dialect-aware verb (Lua IPC after the flip);
            // never raw `hyprctl dispatch`.
            Quickshell.execDetached(["vogix", "hypr", "dispatch", "exit"]);
            return "logout";
        case "suspend":
        case "reboot":
        case "poweroff":
            Quickshell.execDetached(["systemctl", id]);
            return id;
        }
        return "unknown action: " + id;
    }
}

pragma Singleton
// Timed reminders → notifications. Entries persist in state (a restart
// mid-countdown loses nothing); due reminders fire through the shell's own
// notification server and are removed.
import QtQuick
import Quickshell
import Quickshell.Io
import qs.Vogix

Singleton {
    id: root

    // [{ text, at }] — `at` is epoch milliseconds.
    property var entries: []

    function add(text: string, at: real): string {
        root.entries = root.entries.concat([{ text: text, at: at }]);
        save();
        return "reminder set for " + new Date(at).toLocaleString(Qt.locale(), "ddd HH:mm");
    }

    function list(): string {
        if (root.entries.length === 0)
            return "no reminders";
        return root.entries
            .map(e => new Date(e.at).toLocaleString(Qt.locale(), "ddd HH:mm") + "  " + e.text)
            .join("\n");
    }

    function clear(): string {
        root.entries = [];
        save();
        return "cleared";
    }

    function save(): void {
        stateFile.setText(JSON.stringify({ reminders: root.entries }));
    }

    Timer {
        interval: 15000
        running: root.entries.length > 0
        repeat: true
        onTriggered: {
            const now = Date.now();
            const due = root.entries.filter(e => e.at <= now);
            if (due.length === 0)
                return;
            for (const e of due)
                Quickshell.execDetached(["notify-send", "-u", "critical", "Reminder", e.text]);
            root.entries = root.entries.filter(e => e.at > now);
            root.save();
        }
    }

    FileView {
        id: stateFile
        path: Paths.stateRoot + "/desktop/reminders.json"
        watchChanges: false
        onLoaded: {
            try {
                root.entries = JSON.parse(text()).reminders ?? [];
            } catch (e) {}
        }
    }
}

pragma Singleton
// Night light through hyprsunset (Hyprland's own CTM tool): toggling
// starts/stops the process; the state survives shell restarts and re-arms
// hyprsunset on startup when it was left on.
import QtQuick
import Quickshell
import Quickshell.Io
import qs.Vogix

Singleton {
    id: root

    property bool on: false
    readonly property int temperature: (Config.doc.nightlight ?? {}).temperature ?? 4000

    function set(state: bool): string {
        root.on = state;
        apply();
        stateFile.setText(JSON.stringify({ on: root.on }));
        return root.on ? "on" : "off";
    }

    function toggle(): string {
        return set(!root.on);
    }

    function status(): string {
        return root.on ? "on" : "off";
    }

    function apply(): void {
        if (root.on)
            Quickshell.execDetached(["sh", "-c",
                "pkill -x hyprsunset; exec hyprsunset -t " + root.temperature]);
        else
            Quickshell.execDetached(["pkill", "-x", "hyprsunset"]);
    }

    FileView {
        id: stateFile
        path: Paths.stateRoot + "/desktop/nightlight.json"
        watchChanges: false
        onLoaded: {
            try {
                const wanted = JSON.parse(text()).on ?? false;
                if (wanted && !root.on) {
                    root.on = true;
                    root.apply();
                }
            } catch (e) {}
        }
    }
}

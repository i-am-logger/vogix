pragma Singleton
// Stay-awake: a persisted switch that HOLDS every idle stage open. The
// shell is the session's sole idle owner (qs.Services.Idle), so gating its
// monitors is the whole mechanism — no inhibitor protocol dance needed.
import QtQuick
import Quickshell
import Quickshell.Io
import qs.Vogix

Singleton {
    id: root

    property bool on: false

    function set(state: bool): string {
        root.on = state;
        stateFile.setText(JSON.stringify({ on: root.on }));
        return root.on ? "on" : "off";
    }

    function toggle(): string {
        return set(!root.on);
    }

    function status(): string {
        return root.on ? "on" : "off";
    }

    FileView {
        id: stateFile
        path: Paths.stateRoot + "/desktop/stay-awake.json"
        watchChanges: false
        onLoaded: {
            try {
                root.on = JSON.parse(text()).on ?? false;
            } catch (e) {}
        }
    }
}

pragma Singleton
// Weather via wttrbar: refreshed every 30 minutes,
// cached in state so a shell restart shows the last reading immediately.
// The location comes from desktop.json (empty = wttr.in geolocation).
import QtQuick
import Quickshell
import Quickshell.Io
import qs.Vogix

Singleton {
    id: root

    property string text: ""
    property string forecast: ""

    readonly property var conf: Config.doc.weather ?? ({})
    readonly property bool enabled: conf.enable ?? true

    function refresh(): void {
        if (!root.enabled)
            return;
        const loc = root.conf.location ?? "";
        proc.command = loc === ""
            ? ["wttrbar"]
            : ["wttrbar", "--location", loc];
        proc.running = false;
        proc.running = true;
    }

    Timer {
        interval: 30 * 60 * 1000
        running: root.enabled
        repeat: true
        triggeredOnStart: true
        onTriggered: root.refresh()
    }

    Process {
        id: proc
        stdout: StdioCollector {
            onStreamFinished: {
                try {
                    const doc = JSON.parse(text);
                    root.text = doc.text ?? "";
                    root.forecast = (doc.tooltip ?? "").replace(/<[^>]*>/g, "");
                    cache.setText(JSON.stringify({ text: root.text, forecast: root.forecast }));
                } catch (e) {}
            }
        }
    }

    FileView {
        id: cache
        path: Paths.stateRoot + "/desktop/weather.json"
        watchChanges: false
        onLoaded: {
            try {
                const doc = JSON.parse(text());
                if (root.text === "")
                    root.text = doc.text ?? "";
                if (root.forecast === "")
                    root.forecast = doc.forecast ?? "";
            } catch (e) {}
        }
    }
}

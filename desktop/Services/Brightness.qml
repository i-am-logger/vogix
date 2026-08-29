pragma Singleton
// Backlight through brightnessctl: read on demand (panel open), set fires
// the OSD like the volume path does.
import QtQuick
import Quickshell
import Quickshell.Io
import qs.Services

Singleton {
    id: root

    property real level: -1   // 0..1, -1 = unread/no backlight

    function refresh(): void {
        readProc.running = false;
        readProc.running = true;
    }

    function set(value: real): void {
        const pct = Math.round(Math.max(0, Math.min(1, value)) * 100);
        Quickshell.execDetached(["brightnessctl", "set", pct + "%"]);
        root.level = pct / 100;
        Osd.show("brightness", root.level, false, "");
    }

    Process {
        id: readProc
        command: ["brightnessctl", "-m"]
        stdout: StdioCollector {
            onStreamFinished: {
                // machine format: device,class,current,percent%,max
                const m = text.trim().split(",");
                if (m.length >= 5) {
                    const cur = Number(m[2]);
                    const max = Number(m[4]);
                    if (max > 0)
                        root.level = cur / max;
                }
            }
        }
    }
}

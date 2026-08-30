pragma Singleton
// Tailscale link state: `tailscale status --json` on a slow timer, plus
// how long the daemon has been up (systemd's ActiveEnterTimestamp) for
// the "connected N h" readout.
import QtQuick
import Quickshell
import Quickshell.Io

Singleton {
    id: root

    property bool online: false
    property string selfIp: ""
    property string tailnet: ""
    property int peersOnline: 0
    property int peersTotal: 0
    // Epoch ms the daemon came up, 0 = unknown.
    property real sinceEpoch: 0

    function sinceText(nowMs: real): string {
        if (!online || sinceEpoch <= 0)
            return "";
        const mins = Math.max(0, Math.floor((nowMs - sinceEpoch) / 60000));
        if (mins < 60)
            return mins + "m";
        if (mins < 60 * 24)
            return Math.floor(mins / 60) + "h" + (mins % 60 > 0 ? (mins % 60) + "m" : "");
        return Math.floor(mins / (60 * 24)) + "d";
    }

    Timer {
        interval: 30000
        running: true
        repeat: true
        triggeredOnStart: true
        onTriggered: {
            statusProc.running = true;
            sinceProc.running = true;
        }
    }

    Process {
        id: statusProc
        command: ["tailscale", "status", "--json"]

        stdout: StdioCollector {
            onStreamFinished: {
                let doc;
                try {
                    doc = JSON.parse(text);
                } catch (e) {
                    root.online = false;
                    return;
                }
                root.online = doc.Self?.Online ?? false;
                root.selfIp = (doc.TailscaleIPs ?? [])[0] ?? "";
                root.tailnet = doc.CurrentTailnet?.Name ?? "";
                const peers = Object.values(doc.Peer ?? {});
                root.peersTotal = peers.length;
                root.peersOnline = peers.filter(p => p.Online).length;
            }
        }
    }

    Process {
        id: sinceProc
        command: ["systemctl", "show", "tailscaled", "--property=ActiveEnterTimestamp", "--value"]

        stdout: StdioCollector {
            onStreamFinished: {
                // "Fri 2026-08-29 10:11:12 IDT" — parse the numeric core as
                // local time; timezone abbreviations defeat Date.parse.
                const m = text.match(/(\d{4})-(\d{2})-(\d{2}) (\d{2}):(\d{2}):(\d{2})/);
                if (!m) {
                    root.sinceEpoch = 0;
                    return;
                }
                root.sinceEpoch = new Date(
                    Number(m[1]), Number(m[2]) - 1, Number(m[3]),
                    Number(m[4]), Number(m[5]), Number(m[6])).getTime();
            }
        }
    }
}

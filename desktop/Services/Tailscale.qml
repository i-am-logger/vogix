pragma Singleton
// Tailnet state: the link, the peers, and how long the current connection
// has lasted (lib/tailnet.js). Sampled every 30 s by
// data/tailscale-status.sh, because the tailscale CLI has no stable event
// stream: IPN bus notifications reach the CLI only through
// `tailscale debug watch-ipn`, whose output is not an interface.
//
// The connection record lives in the runtime directory, so a shell restart
// inside the session keeps the real start of a connection instead of
// restarting the clock, and a new login or boot does not inherit a stale
// one.
import QtQuick
import Quickshell
import Quickshell.Io
import qs.Vogix
import "lib/tailnet.js" as Tailnet

Singleton {
    id: root

    property int link: Tailnet.Link.Absent
    readonly property bool online: root.link === Tailnet.Link.Connected
    property string selfName: ""
    property string selfIp: ""
    property string tailnet: ""
    property var peers: []
    property int peersOnline: 0
    readonly property int peersTotal: root.peers.length

    // { since, exact, daemonStartMs }; since = 0 while not connected.
    property var connection: ({ since: 0, exact: false, daemonStartMs: 0 })
    property bool _restored: false
    property bool _observed: false

    // How long the current connection has lasted, "" while not connected.
    function sinceText(nowMs: real): string {
        return Tailnet.sinceText(root.connection, nowMs);
    }

    // Link state in words, for the panel.
    function stateText(): string {
        switch (root.link) {
        case Tailnet.Link.Absent:
            return "not installed";
        case Tailnet.Link.DaemonDown:
            return "tailscaled is not running";
        case Tailnet.Link.Stopped:
            return "stopped";
        case Tailnet.Link.LoggedOut:
            return "logged out";
        case Tailnet.Link.Connecting:
            return "connecting";
        case Tailnet.Link.Offline:
            return "offline";
        default:
            return "connected";
        }
    }

    // A sample now, for a panel that just opened.
    function refresh(): void {
        if (root._restored)
            statusProc.running = true;
    }

    function _apply(report): void {
        root.link = report.link;
        root.selfName = report.selfName;
        root.selfIp = report.selfIp;
        root.tailnet = report.tailnet;
        root.peers = report.peers;
        root.peersOnline = report.peersOnline;

        const next = Tailnet.track(root.connection, report, Date.now(), !root._observed);
        root._observed = true;
        const prev = root.connection;
        root.connection = next;
        if (next.since !== prev.since || next.exact !== prev.exact || next.daemonStartMs !== prev.daemonStartMs)
            recordFile.setText(JSON.stringify(next));
    }

    // Sampling starts once the previous shell's record has been read, so
    // the first observation is judged against it.
    Timer {
        interval: 30000
        running: root._restored
        repeat: true
        triggeredOnStart: true
        onTriggered: statusProc.running = true
    }

    FileView {
        id: recordFile

        path: Paths.runtimeRoot + "/desktop/tailscale-connection.json"
        watchChanges: false
        preload: true
        // Absent on the first shell of a session.
        printErrors: false
        onLoaded: {
            root.connection = Tailnet.parseRecord(text());
            root._restored = true;
        }
        onLoadFailed: root._restored = true
    }

    Process {
        id: statusProc

        command: ["sh", Quickshell.shellDir + "/data/tailscale-status.sh"]

        stdout: StdioCollector {
            onStreamFinished: root._apply(Tailnet.parseReport(text))
        }
    }
}

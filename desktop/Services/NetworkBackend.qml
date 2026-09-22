pragma Singleton
// Whether quickshell's Networking has its NetworkManager backend, and the
// way back when it does not. quickshell builds that backend once per
// process, so a shell that starts before NetworkManager is on the system
// bus has no network state for its whole life. Only in that state, a
// dbus-monitor subscription waits for NetworkManager to take its bus name
// (an event, not a poll), and a shell run by systemd then exits with
// status 75, which the vogix-desktop unit answers with a fresh start that
// attaches. A restart due while the session is locked waits for the
// unlock, so the lock surface never drops out from under the user.
import QtQuick
import Quickshell
import Quickshell.Io
import Quickshell.Networking
import qs.Services
import "lib/dbus.js" as DBus

Singleton {
    id: root

    readonly property string service: "org.freedesktop.NetworkManager"
    readonly property bool attached: Networking.backend !== NetworkBackendType.None
    // systemd sets INVOCATION_ID for every unit it runs. An unsupervised
    // shell (a dev `qs -p`, the smoke) has nothing to start it again, so it
    // only reports the missing backend.
    readonly property bool selfRestarts: (Quickshell.env("INVOCATION_ID") ?? "") !== ""
    // NetworkManager has taken its bus name since the shell started
    // without it.
    property bool appeared: false
    property var _parse: DBus.initialState()

    function _onAppeared(): void {
        if (root.appeared)
            return;
        root.appeared = true;
        root._restartIfUnlocked();
    }

    function _restartIfUnlocked(): void {
        if (!root.appeared || Lock.locked)
            return;
        console.info("NetworkBackend: NetworkManager appeared after the shell started; restarting to attach");
        Qt.exit(75);
    }

    Connections {
        target: Lock

        function onLockedChanged(): void {
            root._restartIfUnlocked();
        }
    }

    Process {
        id: watchProc

        running: !root.attached && root.selfRestarts && !root.appeared
        command: ["dbus-monitor", "--system", DBus.ownerChangeRule(root.service)]

        stdout: SplitParser {
            onRead: line => {
                const r = DBus.step(root._parse, line);
                root._parse = r.state;
                if (r.event === null)
                    return;
                if (r.event.kind === "subscribed")
                    presenceProc.running = true;
                else if (r.event.name === root.service && r.event.newOwner !== "")
                    root._onAppeared();
            }
        }

        onRunningChanged: {
            if (!running && !root.attached && root.selfRestarts && !root.appeared)
                console.warn("NetworkBackend: dbus-monitor exited; the shell will not reattach to NetworkManager on its own");
        }
    }

    // Closes the gap between quickshell giving up on the backend and the
    // subscription taking effect: asked once, as soon as the monitor is
    // live.
    Process {
        id: presenceProc

        command: ["dbus-send", "--system", "--print-reply", "--dest=org.freedesktop.DBus",
            "/org/freedesktop/DBus", "org.freedesktop.DBus.NameHasOwner", "string:" + root.service]

        stdout: StdioCollector {
            onStreamFinished: {
                if (DBus.hasOwnerReply(text))
                    root._onAppeared();
            }
        }
    }
}

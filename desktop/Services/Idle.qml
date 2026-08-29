pragma Singleton
// Idle staging (ext-idle-notify): screensaver → dim → lock → screens off →
// suspend, each stage its own monitor with its own desktop.json timeout
// (null disables). respectInhibitors everywhere: a video or an inhibitor
// holds all stages; the stay-awake switch holds them all too (the shell is
// the session's sole idle owner, so gating its monitors IS the mechanism).
import QtQuick
import Quickshell
import Quickshell.Wayland
import qs.Services
import qs.Vogix

Singleton {
    id: root

    readonly property var conf: Config.doc.idle ?? ({})
    readonly property bool held: StayAwake.on
    property bool dimmed: false
    property bool screensaverActive: false

    IdleMonitor {
        enabled: !root.held && (root.conf.screensaver ?? null) !== null
        timeout: root.conf.screensaver ?? 0
        respectInhibitors: true
        // Any input resets seat idle, which flips this back off — the
        // overlay needs no dismiss handling of its own.
        onIsIdleChanged: root.screensaverActive = isIdle
    }

    IdleMonitor {
        enabled: !root.held && (root.conf.dim ?? null) !== null
        timeout: root.conf.dim ?? 0
        respectInhibitors: true
        onIsIdleChanged: root.dimmed = isIdle
    }

    IdleMonitor {
        enabled: !root.held && (root.conf.lock ?? null) !== null
        timeout: root.conf.lock ?? 0
        respectInhibitors: true
        onIsIdleChanged: {
            if (isIdle)
                Lock.lock();
        }
    }

    IdleMonitor {
        enabled: !root.held && (root.conf.screenOff ?? null) !== null
        timeout: root.conf.screenOff ?? 0
        respectInhibitors: true
        onIsIdleChanged: {
            // Dialect-correct on both config engines via the vogix verb.
            Quickshell.execDetached(["vogix", "hypr", "dispatch", isIdle ? "dpms, off" : "dpms, on"]);
        }
    }

    IdleMonitor {
        enabled: !root.held && (root.conf.suspend ?? null) !== null
        timeout: root.conf.suspend ?? 0
        respectInhibitors: true
        onIsIdleChanged: {
            if (isIdle)
                Quickshell.execDetached(["systemctl", "suspend"]);
        }
    }
}

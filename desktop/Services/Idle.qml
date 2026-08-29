pragma Singleton
// Idle staging (ext-idle-notify): dim → lock → screens off → suspend, each
// stage its own monitor with its own desktop.json timeout (null disables).
// respectInhibitors everywhere: a video or an inhibitor holds all stages.
import QtQuick
import Quickshell
import Quickshell.Wayland
import qs.Vogix

Singleton {
    id: root

    readonly property var conf: Config.doc.idle ?? ({})
    property bool dimmed: false

    IdleMonitor {
        enabled: (root.conf.dim ?? null) !== null
        timeout: root.conf.dim ?? 0
        respectInhibitors: true
        onIsIdleChanged: root.dimmed = isIdle
    }

    IdleMonitor {
        enabled: (root.conf.lock ?? null) !== null
        timeout: root.conf.lock ?? 0
        respectInhibitors: true
        onIsIdleChanged: {
            if (isIdle)
                Lock.lock();
        }
    }

    IdleMonitor {
        enabled: (root.conf.screenOff ?? null) !== null
        timeout: root.conf.screenOff ?? 0
        respectInhibitors: true
        onIsIdleChanged: {
            // Dialect-correct on both config engines via the vogix verb.
            Quickshell.execDetached(["vogix", "hypr", "dispatch", isIdle ? "dpms, off" : "dpms, on"]);
        }
    }

    IdleMonitor {
        enabled: (root.conf.suspend ?? null) !== null
        timeout: root.conf.suspend ?? 0
        respectInhibitors: true
        onIsIdleChanged: {
            if (isIdle)
                Quickshell.execDetached(["systemctl", "suspend"]);
        }
    }
}

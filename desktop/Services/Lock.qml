pragma Singleton
// The session lock: ONE WlSessionLock and ONE PamContext for the whole
// session (surfaces render per screen but authentication is singular).
// Locking REFUSES when the PAM service file is absent — never an
// unlockable screen. `secure` is the compositor's own confirmation that
// every output is covered.
import QtQuick
import Quickshell
import Quickshell.Io
import Quickshell.Wayland
import Quickshell.Services.Pam
import qs.Lock
import qs.Vogix

Singleton {
    id: root

    readonly property var conf: Config.doc.lock ?? ({})
    readonly property string pamService: conf.pamService ?? "vogix-lock"
    property bool pamPresent: false

    readonly property bool locked: sessionLock.locked
    readonly property bool secure: sessionLock.secure

    // Auth state shared by every per-screen surface.
    property string authMessage: ""
    property bool authError: false
    property bool inProgress: false
    property var pendingResponse: null

    function lock(): string {
        if (!(conf.enable ?? true))
            return "refused: lock disabled in desktop.json";
        if (!root.pamPresent) {
            console.warn("vogix: refusing to lock — /etc/pam.d/" + root.pamService
                + " is not configured (an unlockable screen is worse than an unlocked one)");
            return "refused: PAM service '" + root.pamService + "' not configured";
        }
        sessionLock.locked = true;
        return "locking";
    }

    function status(): string {
        return root.secure ? "secure" : (root.locked ? "locked" : "unlocked");
    }

    function submit(password: string): void {
        if (root.inProgress)
            return;
        root.authError = false;
        root.authMessage = "";
        root.inProgress = true;
        root.pendingResponse = password;
        pam.start();
    }

    FileView {
        path: "/etc/pam.d/" + root.pamService
        watchChanges: false
        onLoaded: root.pamPresent = true
        onLoadFailed: root.pamPresent = false
    }

    PamContext {
        id: pam
        config: root.pamService

        onResponseRequiredChanged: {
            if (responseRequired && root.pendingResponse !== null) {
                respond(root.pendingResponse);
                root.pendingResponse = null;
            }
        }

        onMessageChanged: {
            if (message !== "") {
                root.authMessage = message;
                root.authError = messageIsError;
            }
        }

        onCompleted: result => {
            root.inProgress = false;
            root.pendingResponse = null;
            if (result === PamResult.Success) {
                root.authMessage = "";
                root.authError = false;
                sessionLock.locked = false;
            } else {
                // Wrong password / PAM failure: the screen STAYS locked.
                if (root.authMessage === "") {
                    root.authMessage = "Authentication failed";
                    root.authError = true;
                }
            }
        }
    }

    WlSessionLock {
        id: sessionLock

        LockSurface {}
    }
}

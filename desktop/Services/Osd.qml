pragma Singleton
// On-screen display state: what to flash (volume/mic/brightness/caps/custom),
// fed both REACTIVELY (Audio watches the default sink) and by the
// `vogix desktop osd` push verb over IPC. The window renders whatever is
// here while `visible` holds; the timer retracts it.
import QtQuick
import Quickshell
import qs.Vogix

Singleton {
    id: root

    property string kind: "volume"
    property real value: 0 // 0..1, -1 = no gauge
    property bool muted: false
    property string message: ""
    property bool visible: false

    function show(kind, value, muted, message) {
        root.kind = kind;
        root.value = value;
        root.muted = muted;
        root.message = message;
        root.visible = true;
        hideTimer.restart();
    }

    Timer {
        id: hideTimer
        interval: (Config.doc.osd ?? {}).timeout ?? 1500
        onTriggered: root.visible = false
    }
}

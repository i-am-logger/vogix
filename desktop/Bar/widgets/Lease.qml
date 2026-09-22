// A widget's hold on a ref-counted data source (a tap, a peak monitor, a
// sampler): `acquire` fires when `active` turns true, `release` when it
// turns false or the lease is destroyed while held — so a widget can
// neither leak a reference nor count one twice. Widgets bind `active` to
// their bar's `live`.
//
// Plain QtQuick in a module with no quickshell singletons, so
// qmltestrunner pins it (tests/desktop/tst_lease.qml).
import QtQuick

QtObject {
    id: root

    property bool active: false

    signal acquire()
    signal release()

    property bool _held: false

    function _sync(): void {
        if (root.active === root._held)
            return;
        root._held = root.active;
        if (root._held)
            root.acquire();
        else
            root.release();
    }

    onActiveChanged: root._sync()
    Component.onCompleted: root._sync()
    Component.onDestruction: {
        if (root._held) {
            root._held = false;
            root.release();
        }
    }
}

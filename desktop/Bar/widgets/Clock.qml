import QtQuick
import Quickshell
import qs.Bar.widgets
import qs.Services

BarText {
    SystemClock {
        id: clock
        precision: SystemClock.Minutes
    }

    // ISO date, 24h — the HUD reads like a log line.
    text: axis?.vertical
        ? Qt.formatDateTime(clock.date, "HH\nmm")
        : Qt.formatDateTime(clock.date, "yyyy-MM-dd  HH:mm")
    horizontalAlignment: Text.AlignHCenter

    MouseArea {
        anchors.fill: parent
        onClicked: Panels.toggle("calendar")
    }
}

import QtQuick
import Quickshell
import qs.Bar.widgets
import qs.Services

BarText {
    SystemClock {
        id: clock
        precision: SystemClock.Minutes
    }

    text: axis?.vertical
        ? Qt.formatDateTime(clock.date, "HH\nmm")
        : Qt.formatDateTime(clock.date, "ddd d MMM  HH:mm")
    horizontalAlignment: Text.AlignHCenter

    MouseArea {
        anchors.fill: parent
        onClicked: Panels.toggle("calendar")
    }
}

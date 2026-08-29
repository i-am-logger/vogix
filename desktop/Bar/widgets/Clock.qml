import QtQuick
import Quickshell
import qs.Bar.widgets
import qs.Vogix

BarText {
    SystemClock {
        id: clock
        precision: SystemClock.Minutes
    }

    text: Qt.formatDateTime(clock.date, "ddd d MMM  HH:mm")
}

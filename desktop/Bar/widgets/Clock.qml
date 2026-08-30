// The CLOCK cell: big bold HH:mm:ss, ticking seconds — the HUD's
// heartbeat. Stacked HH/mm on a vertical rail. Click opens the calendar.
import QtQuick
import Quickshell
import qs.Bar.widgets
import qs.Services
import qs.Vogix

FrameCell {
    id: root

    property BarAxis axis: null
    readonly property bool vertical: axis?.vertical ?? false

    title: "CLOCK"
    padH: 12
    padV: 2

    SystemClock {
        id: clock
        precision: root.vertical ? SystemClock.Minutes : SystemClock.Seconds
    }

    BarText {
        text: root.vertical
            ? Qt.formatDateTime(clock.date, "HH\nmm")
            : Qt.formatDateTime(clock.date, "HH:mm:ss")
        font.pixelSize: Metrics.title
        font.bold: true
        horizontalAlignment: Text.AlignHCenter
        color: Theme.semantic.foreground_bright ?? Tokens.color("bar", "foreground")
    }

    MouseArea {
        anchors.fill: parent
        onClicked: Panels.toggle("calendar")
    }
}

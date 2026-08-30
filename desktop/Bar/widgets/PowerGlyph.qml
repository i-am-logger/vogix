// The power menu button — PWR rotated on the rail, a glyph elsewhere.
import QtQuick
import qs.Bar.widgets
import qs.Services
import qs.Vogix

Item {
    id: root

    property BarAxis axis: null
    readonly property bool vertical: axis?.vertical ?? false

    implicitWidth: vertical ? t.implicitHeight : t.implicitWidth
    implicitHeight: vertical ? t.implicitWidth : t.implicitHeight

    BarText {
        id: t
        anchors.centerIn: parent
        rotation: root.vertical ? 90 : 0
        text: root.vertical ? "PWR" : "󰐥"
        font.pixelSize: root.vertical ? Metrics.micro : Metrics.body
        font.letterSpacing: root.vertical ? 1.5 : 0
        color: Tokens.color("bar", "muted")
    }

    MouseArea {
        anchors.fill: parent
        onClicked: Power.show()
    }
}

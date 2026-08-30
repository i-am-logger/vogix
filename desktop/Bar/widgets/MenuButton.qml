// The root menu button — MNU rotated on the rail, a glyph on a
// horizontal bar. The touch/pointer way into `vogix desktop menu`.
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
        text: root.vertical ? "MNU" : "󰍜"
        font.pixelSize: root.vertical ? Metrics.micro : Metrics.body
        font.letterSpacing: root.vertical ? 1.5 : 0
        color: Tokens.color("bar", "muted")
    }

    MouseArea {
        anchors.fill: parent
        onClicked: Launcher.openMenu("")
    }
}

// The Flight Deck framed cell: a hairline square frame whose uppercase
// title BREAKS the top border (the title chip carries the surface
// background so the border reads as cut). Content goes in the default
// slot; the caller passes surface-resolved colors.
import QtQuick
import qs.Vogix

Item {
    id: root

    property string title: ""
    property color borderColor: "#ff00ff"
    property color titleColor: borderColor
    // MUST match the surface behind the panel — it is what cuts the border.
    property color chipColor: "#ff00ff"
    property real contentPadding: Metrics.unit * 2

    default property alias content: slot.data

    implicitWidth: slot.childrenRect.width + contentPadding * 2
    implicitHeight: slot.childrenRect.height + contentPadding * 2 + titleText.height / 2

    Rectangle {
        id: frame
        anchors.fill: parent
        anchors.topMargin: titleText.height / 2
        color: "transparent"
        border.width: 1
        border.color: root.borderColor
    }

    Rectangle {
        anchors.verticalCenter: frame.top
        anchors.left: frame.left
        anchors.leftMargin: Metrics.unit * 2
        width: titleText.width + Metrics.unit * 2
        height: titleText.height
        color: root.chipColor
        visible: root.title !== ""

        HudLabel {
            id: titleText
            anchors.centerIn: parent
            text: root.title
            font.pixelSize: Metrics.micro
            color: root.titleColor
        }
    }

    Item {
        id: slot
        anchors.fill: frame
        anchors.margins: root.contentPadding
    }
}

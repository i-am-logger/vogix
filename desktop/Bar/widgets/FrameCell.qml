// THE Flight Deck cell: a hairline frame whose micro-caps title BREAKS
// the top border. The border is drawn as segments with a real gap under
// the title (no background chip), so it works over the bar's translucent
// ground. Content goes in the default slot.
import QtQuick
import qs.Vogix

Item {
    id: root

    property string title: ""
    property color frameColor: Tokens.color("meter", "frame")
    property color titleColor: Tokens.color("meter", "label")
    property real padH: 8
    property real padV: 4

    default property alias content: slot.data

    implicitWidth: slot.childrenRect.width + padH * 2
    implicitHeight: slot.childrenRect.height + padV * 2 + titleText.height / 2

    readonly property real frameTop: titleText.height / 2
    readonly property real gapStart: 8
    readonly property real gapEnd: gapStart + (root.title === "" ? 0 : titleText.width + 6)

    // Top border, split around the title.
    Rectangle {
        x: 0
        y: root.frameTop
        width: root.title === "" ? parent.width : root.gapStart - 3
        height: 1
        color: root.frameColor
    }

    Rectangle {
        visible: root.title !== ""
        x: root.gapEnd
        y: root.frameTop
        width: Math.max(0, parent.width - root.gapEnd)
        height: 1
        color: root.frameColor
    }

    Rectangle {
        x: 0
        y: root.frameTop
        width: 1
        height: parent.height - root.frameTop
        color: root.frameColor
    }

    Rectangle {
        x: parent.width - 1
        y: root.frameTop
        width: 1
        height: parent.height - root.frameTop
        color: root.frameColor
    }

    Rectangle {
        x: 0
        y: parent.height - 1
        width: parent.width
        height: 1
        color: root.frameColor
    }

    Text {
        id: titleText
        x: root.gapStart + 3
        y: 0
        text: root.title
        color: root.titleColor
        font.family: Config.fontFamily
        font.pixelSize: Metrics.micro
        font.letterSpacing: 1.5
        font.capitalization: Font.AllUppercase
    }

    Item {
        id: slot
        x: root.padH
        y: root.frameTop + root.padV
        width: parent.width - root.padH * 2
        height: parent.height - root.frameTop - root.padV * 2
    }
}

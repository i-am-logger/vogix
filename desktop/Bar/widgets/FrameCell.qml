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

    // Whole-cell click target. Widgets must use THIS, never their own
    // anchors.fill MouseArea inside the slot — a filling child feeds the
    // slot's childrenRect back into the cell's implicit size (binding
    // loop, caught live).
    property bool interactive: false

    signal clicked()

    default property alias content: slot.data

    readonly property real frameTop: titleText.height / 2
    // The title's bottom half hangs BELOW the border line — content must
    // clear it, not just the border.
    readonly property real contentTop: frameTop
        + (root.title === "" ? padV : Math.max(padV, titleText.height / 2 + 2))
    readonly property real gapStart: 8
    readonly property real gapEnd: gapStart + (root.title === "" ? 0 : titleText.width + 6)

    implicitWidth: Math.max(slot.childrenRect.width + padH * 2, gapEnd + 8)
    implicitHeight: slot.childrenRect.height + contentTop + padV

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
        y: root.contentTop
        width: parent.width - root.padH * 2
        height: parent.height - root.contentTop - root.padV
    }

    MouseArea {
        anchors.fill: parent
        enabled: root.interactive
        // Below the slot, so a widget's own inner mouse areas win.
        z: -1
        onClicked: root.clicked()
    }
}

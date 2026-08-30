pragma ComponentBehavior: Bound
// Notification popups: a top-right column on the focused monitor. Cards use
// the `notification` surface tokens; the per-app accent rule recolors the
// left edge; critical cards never auto-expire; DND hides everything except
// critical and rule-exempt apps.
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Hyprland
import qs.Services
import qs.Vogix

PanelWindow {
    id: root

    readonly property var visiblePopups:
        Notifs.popups.filter(p => Notifs.visibleUnderDnd(p))
            .slice(-((Config.doc.notifications ?? {}).maxVisible ?? 5))

    // Follow the focused monitor; fall back to the first screen.
    screen: {
        const name = Hyprland.focusedMonitor?.name ?? "";
        return Quickshell.screens.find(s => s.name === name) ?? Quickshell.screens[0] ?? null;
    }

    visible: visiblePopups.length > 0
    anchors {
        top: true
        right: true
    }
    margins.top: 8
    margins.right: 8
    implicitWidth: 380
    implicitHeight: column.implicitHeight
    color: "transparent"
    exclusionMode: ExclusionMode.Ignore

    ColumnLayout {
        id: column
        anchors.fill: parent
        spacing: 8

        Repeater {
            model: root.visiblePopups

            Rectangle {
                id: card

                required property var modelData

                Layout.fillWidth: true
                implicitHeight: content.implicitHeight + 20
                radius: 8
                color: Tokens.color("notification", "background")
                border.width: 1
                border.color: card.modelData.urgency === 2
                    ? Tokens.color("notification", "urgent")
                    : Tokens.color("notification", "border")

                Rectangle {
                    width: 4
                    radius: 2
                    anchors {
                        left: parent.left
                        top: parent.top
                        bottom: parent.bottom
                        margins: 4
                    }
                    color: card.modelData.accent && Theme.semantic[card.modelData.accent]
                        ? Theme.semantic[card.modelData.accent]
                        : (card.modelData.urgency === 2
                            ? Tokens.color("notification", "urgent")
                            : Tokens.color("notification", "accent"))
                }

                ColumnLayout {
                    id: content
                    anchors {
                        fill: parent
                        leftMargin: 16
                        rightMargin: 10
                        topMargin: 10
                        bottomMargin: 10
                    }
                    spacing: 4

                    Text {
                        Layout.fillWidth: true
                        text: card.modelData.summary
                        color: Tokens.color("notification", "foreground")
                        font.family: Config.fontFamily
                        font.pixelSize: Metrics.body
                        font.bold: true
                        elide: Text.ElideRight
                    }

                    Text {
                        Layout.fillWidth: true
                        visible: card.modelData.body !== ""
                        text: card.modelData.body
                        color: Tokens.color("notification", "foreground")
                        font.family: Config.fontFamily
                        font.pixelSize: Metrics.bodySmall
                        wrapMode: Text.Wrap
                        maximumLineCount: 4
                        elide: Text.ElideRight
                    }

                    Text {
                        text: card.modelData.appName
                        color: Tokens.color("notification", "muted")
                        font.family: Config.fontFamily
                        font.pixelSize: Metrics.caption
                    }
                }

                MouseArea {
                    anchors.fill: parent
                    onClicked: Notifs.dismiss(card.modelData.key)
                }

                Timer {
                    interval: Math.max(1, card.modelData.timeout)
                    running: card.modelData.timeout > 0
                    onTriggered: Notifs.expire(card.modelData.key)
                }
            }
        }
    }
}

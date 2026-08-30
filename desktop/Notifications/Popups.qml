pragma ComponentBehavior: Bound
// Notification popups, Flight Deck form: a top-right column of ALERT
// cards on the focused monitor. Square corners, hairline border; a
// CRITICAL card swaps the hairline for a dashed danger frame and never
// auto-expires. The header is the log line — APP NAME in micro caps and
// an HH:mm:ss stamp; the 1px drain bar along the bottom shows the real
// remaining lifetime (it resets exactly when the expiry timer does).
// Overflow past maxVisible queues, announced by the +N QUEUED line.
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Hyprland
import qs.Components
import qs.Services
import qs.Vogix

PanelWindow {
    id: root

    readonly property var eligible: Notifs.popups.filter(p => Notifs.visibleUnderDnd(p))
    readonly property var visiblePopups:
        eligible.slice(-((Config.doc.notifications ?? {}).maxVisible ?? 5))
    readonly property int queued: eligible.length - visiblePopups.length

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
    implicitWidth: 440
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
                readonly property bool critical: card.modelData.urgency === 2
                readonly property color accentColor:
                    card.modelData.accent && Theme.semantic[card.modelData.accent]
                        ? Theme.semantic[card.modelData.accent]
                        : (card.critical
                            ? Tokens.color("notification", "urgent")
                            : Tokens.color("notification", "accent"))

                Layout.fillWidth: true
                implicitHeight: content.implicitHeight + 22
                color: Tokens.color("notification", "background")
                border.width: card.critical ? 0 : 1
                border.color: Tokens.color("notification", "border")

                ScanlineOverlay {}

                DashedBorder {
                    visible: card.critical
                    color: Tokens.color("notification", "urgent")
                }

                Rectangle {
                    width: 3
                    anchors {
                        left: parent.left
                        top: parent.top
                        bottom: parent.bottom
                    }
                    color: card.accentColor
                }

                ColumnLayout {
                    id: content
                    anchors {
                        fill: parent
                        leftMargin: 15
                        rightMargin: 12
                        topMargin: 10
                        bottomMargin: 12
                    }
                    spacing: 4

                    RowLayout {
                        Layout.fillWidth: true
                        spacing: 8

                        HudLabel {
                            text: (card.critical ? "▲ " : "") + card.modelData.appName
                            font.pixelSize: Metrics.micro
                            color: card.critical
                                ? Tokens.color("notification", "urgent")
                                : Tokens.color("notification", "muted")
                        }

                        Item { Layout.fillWidth: true }

                        Text {
                            text: Qt.formatTime(new Date(card.modelData.at ?? Date.now()), "HH:mm:ss")
                            color: Tokens.color("notification", "muted")
                            font.family: Config.fontFamily
                            font.pixelSize: Metrics.micro
                        }
                    }

                    Text {
                        Layout.fillWidth: true
                        text: card.modelData.summary
                        color: Tokens.color("notification", "foreground")
                        font.family: Config.fontFamily
                        font.pixelSize: Metrics.subtitle
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
                        maximumLineCount: 6
                        elide: Text.ElideRight
                    }
                }

                // The lifetime made visible: 1px, full width at arrival,
                // empty at expiry. Restarts with the Timer below when the
                // card is recreated, so it never lies about remaining time.
                Rectangle {
                    property real drainFrac: 1

                    visible: card.modelData.timeout > 0
                    anchors.left: parent.left
                    anchors.bottom: parent.bottom
                    height: 1
                    width: parent.width * drainFrac
                    color: card.accentColor

                    NumberAnimation on drainFrac {
                        from: 1
                        to: 0
                        duration: Math.max(1, card.modelData.timeout)
                        running: card.modelData.timeout > 0
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

        HudLabel {
            visible: root.queued > 0
            Layout.alignment: Qt.AlignRight
            text: "+" + root.queued + " QUEUED"
            font.pixelSize: Metrics.micro
            color: Tokens.color("notification", "muted")
        }
    }
}

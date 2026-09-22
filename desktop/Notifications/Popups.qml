pragma ComponentBehavior: Bound
// Notification popups, Flight Deck form: a top-right column of framed
// cards on the focused monitor whose title BREAKS the top border —
// "APPNAME" on a faint hairline frame, "ALERT :: APPNAME" on a solid
// danger frame when critical (critical never auto-expires). The 3px
// drain bar along the bottom runs on a track and shows the card's real
// remaining lifetime (it resets exactly when the expiry timer does).
// Overflow past maxVisible queues, announced by the +N QUEUED line.
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Hyprland
import qs.Bar.widgets
import qs.Components
import qs.Geometry
import qs.Services
import qs.Vogix

PanelWindow {
    id: root

    readonly property var eligible: Notifs.popups.filter(p => Notifs.visibleUnderDnd(p))
    readonly property var visiblePopups:
        eligible.slice(-((Config.doc.notifications ?? {}).maxVisible ?? 5))
    readonly property int queued: eligible.length - visiblePopups.length

    // exclusionMode Ignore places the column from the raw screen edge, so
    // every bar's live thickness (0 when off or parked) is an inset: the
    // column takes the top-right corner of what the bars leave free.
    readonly property rect area: Placement.freeArea(
        Qt.size(root.screen?.width ?? 0, root.screen?.height ?? 0),
        BarState.thickness("top"), BarState.thickness("bottom"),
        BarState.thickness("left"), BarState.thickness("right"))
    readonly property point origin: Placement.popupOrigin(
        "", Qt.rect(0, 0, 0, 0), Qt.size(root.implicitWidth, root.implicitHeight), root.area)

    // Follow the focused monitor; fall back to the first screen.
    screen: {
        const name = Hyprland.focusedMonitor?.name ?? "";
        return Quickshell.screens.find(s => s.name === name) ?? Quickshell.screens[0] ?? null;
    }

    visible: visiblePopups.length > 0
    anchors {
        top: true
        left: true
    }
    margins.top: origin.y
    margins.left: origin.x
    implicitWidth: 440
    // Never taller than the free area. The column hangs from the window's
    // bottom, so past that height the OLDEST cards run off the top and the
    // newest card and the queue line stay on screen.
    implicitHeight: Math.min(column.implicitHeight, area.height)
    color: "transparent"
    exclusionMode: ExclusionMode.Ignore

    ColumnLayout {
        id: column
        anchors {
            left: parent.left
            right: parent.right
            bottom: parent.bottom
        }
        spacing: 8

        Repeater {
            model: root.visiblePopups

            Item {
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
                implicitHeight: frame.implicitHeight + 3

                Rectangle {
                    anchors.fill: frame
                    anchors.topMargin: frame.frameTop
                    color: Tokens.color("notification", "background")
                }

                FrameCell {
                    id: frame

                    anchors.left: parent.left
                    anchors.right: parent.right
                    title: (card.critical ? "ALERT :: " : "") + card.modelData.appName
                    titleColor: card.critical
                        ? Tokens.color("notification", "urgent")
                        : Tokens.color("notification", "muted")
                    frameColor: card.critical
                        ? Tokens.color("notification", "urgent")
                        : Tokens.color("notification", "border")
                    padH: 16
                    padV: 12

                    // Sized from the card, whose width the column imposes
                    // — never from the slot (see FrameCell).
                    ColumnLayout {
                        width: card.width - frame.padH * 2
                        spacing: 5

                        RowLayout {
                            Layout.fillWidth: true
                            spacing: 8

                            Text {
                                Layout.fillWidth: true
                                text: card.modelData.summary
                                color: Theme.semantic.foreground_bright ?? Tokens.color("notification", "foreground")
                                font.family: Config.fontFamily
                                font.pixelSize: Metrics.subtitle
                                font.bold: true
                                elide: Text.ElideRight
                            }

                            // Arrival time; a card restored from a state
                            // file that never recorded one says so rather
                            // than stamping the restore.
                            Text {
                                text: typeof card.modelData.at === "number"
                                    ? Qt.formatTime(new Date(card.modelData.at), "HH:mm:ss")
                                    : "--:--:--"
                                color: Tokens.color("notification", "muted")
                                font.family: Config.fontFamily
                                font.pixelSize: Metrics.micro
                            }
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
                }

                // The lifetime made visible: a 3px fill draining along a
                // faint track. Restarts with the Timer below when the card
                // is recreated, so it never lies about remaining time.
                Rectangle {
                    visible: card.modelData.timeout > 0
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.bottom: parent.bottom
                    height: 3
                    color: Qt.alpha(Tokens.color("notification", "muted"), 0.35)

                    Rectangle {
                        property real drainFrac: 1

                        anchors.left: parent.left
                        height: parent.height
                        width: parent.width * drainFrac
                        color: card.accentColor

                        NumberAnimation on drainFrac {
                            from: 1
                            to: 0
                            duration: Math.max(1, card.modelData.timeout)
                            running: card.modelData.timeout > 0
                        }
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

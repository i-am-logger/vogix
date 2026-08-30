pragma ComponentBehavior: Bound
// One bar per monitor. Hiding PARKS the layer past the screen edge instead
// of unmapping it (remapping a layer costs ~150 ms, a slide costs
// ~20 ms and keeps the widgets warm).
import QtQuick
import QtQuick.Layouts
import Quickshell
import qs.Vogix

Scope {
    id: root

    property bool hidden: false

    Variants {
        model: Quickshell.screens

        PanelWindow {
            id: panel

            required property var modelData
            readonly property bool onTop: (Config.bar.position ?? "top") === "top"
            readonly property int barHeight: Config.bar.height ?? 32

            screen: modelData
            visible: (Config.bar.enable ?? true)

            anchors {
                left: true
                right: true
                top: onTop
                bottom: !onTop
            }

            implicitHeight: barHeight
            exclusiveZone: root.hidden ? 0 : barHeight
            margins.top: root.hidden && onTop ? -barHeight : 0
            margins.bottom: root.hidden && !onTop ? -barHeight : 0
            color: "transparent"

            Rectangle {
                anchors.fill: parent
                color: Tokens.color("bar", "background")

                RowLayout {
                    anchors.fill: parent
                    anchors.leftMargin: 8
                    anchors.rightMargin: 8
                    spacing: 12

                    Section {
                        names: Config.bar.layout ? (Config.bar.layout.left ?? []) : []
                        Layout.alignment: Qt.AlignLeft
                    }
                    Item { Layout.fillWidth: true }
                    Section {
                        names: Config.bar.layout ? (Config.bar.layout.center ?? []) : []
                        Layout.alignment: Qt.AlignHCenter
                    }
                    Item { Layout.fillWidth: true }
                    Section {
                        names: Config.bar.layout ? (Config.bar.layout.right ?? []) : []
                        Layout.alignment: Qt.AlignRight
                    }
                }
            }
        }
    }
}

pragma ComponentBehavior: Bound
// The status-notifier tray: one icon per item; left click activates,
// middle click secondary-activates, right click toggles the item's own
// menu (activates an item that has none). The menu is anchored to its
// icon through the compositor's popup positioner and opens away from the
// bar's screen edge.
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Services.SystemTray
import Quickshell.Widgets

GridLayout {
    id: root

    property BarAxis axis: null

    // The icon side facing the screen interior: the menu anchors there and
    // grows the same way.
    readonly property int menuSide: {
        switch (axis?.edge ?? "top") {
        case "bottom": return Edges.Top;
        case "left": return Edges.Right;
        case "right": return Edges.Left;
        default: return Edges.Bottom;
        }
    }

    flow: (axis?.vertical ?? false) ? GridLayout.TopToBottom : GridLayout.LeftToRight
    rowSpacing: 8
    columnSpacing: 8

    Repeater {
        model: SystemTray.items

        IconImage {
            id: trayIcon

            required property SystemTrayItem modelData

            implicitSize: 18
            source: trayIcon.modelData.icon
            Layout.alignment: Qt.AlignVCenter

            QsMenuAnchor {
                id: menuAnchor

                menu: trayIcon.modelData.menu
                anchor.item: trayIcon

                // The positioner reads the anchor when the menu opens, so
                // the side is set then: the bar's axis arrives after load.
                function toggle(): void {
                    if (menuAnchor.visible) {
                        menuAnchor.close();
                        return;
                    }
                    menuAnchor.anchor.edges = root.menuSide;
                    menuAnchor.anchor.gravity = root.menuSide;
                    menuAnchor.open();
                }
            }

            MouseArea {
                anchors.fill: parent
                acceptedButtons: Qt.LeftButton | Qt.RightButton | Qt.MiddleButton
                onClicked: mouse => {
                    if (mouse.button === Qt.RightButton && trayIcon.modelData.hasMenu) {
                        menuAnchor.toggle();
                    } else if (mouse.button === Qt.MiddleButton) {
                        trayIcon.modelData.secondaryActivate();
                    } else {
                        trayIcon.modelData.activate();
                    }
                }
                onWheel: wheel => {
                    trayIcon.modelData.scroll(wheel.angleDelta.y, false);
                }
            }
        }
    }
}

pragma ComponentBehavior: Bound
// The status-notifier tray: one icon per item; left click activates,
// right click shows the item's own menu.
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Services.SystemTray
import Quickshell.Widgets

GridLayout {
    id: root

    property BarAxis axis: null

    flow: (axis?.vertical ?? false) ? GridLayout.TopToBottom : GridLayout.LeftToRight
    rowSpacing: 8
    columnSpacing: 8

    Repeater {
        model: SystemTray.items

        IconImage {
            id: trayIcon

            required property var modelData

            implicitSize: 18
            source: trayIcon.modelData.icon
            Layout.alignment: Qt.AlignVCenter

            MouseArea {
                anchors.fill: parent
                acceptedButtons: Qt.LeftButton | Qt.RightButton | Qt.MiddleButton
                onClicked: mouse => {
                    if (mouse.button === Qt.RightButton && trayIcon.modelData.hasMenu)
                        trayIcon.modelData.display(trayIcon.QsWindow.window, trayIcon.x, trayIcon.y);
                    else if (mouse.button === Qt.MiddleButton)
                        trayIcon.modelData.secondaryActivate();
                    else
                        trayIcon.modelData.activate();
                }
                onWheel: wheel => {
                    trayIcon.modelData.scroll(wheel.angleDelta.y, false);
                }
            }
        }
    }
}

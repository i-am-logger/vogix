pragma ComponentBehavior: Bound
// The clock's month popup: a navigable month grid, today accented.
import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import qs.Panels
import qs.Services
import qs.Vogix

ColumnLayout {
    id: root

    property date shown: new Date()

    function shift(months: int): void {
        const d = new Date(root.shown);
        d.setMonth(d.getMonth() + months);
        root.shown = d;
    }

    spacing: 8

    RowLayout {
        Layout.fillWidth: true

        PanelLabel {
            text: "󰅁"
            color: Tokens.color("popup", "accent")

            MouseArea {
                anchors.fill: parent
                onClicked: root.shift(-1)
            }
        }

        PanelLabel {
            Layout.fillWidth: true
            horizontalAlignment: Text.AlignHCenter
            text: Qt.formatDate(root.shown, "MMMM yyyy")
            font.bold: true
            color: Tokens.color("popup", "accent")
        }

        PanelLabel {
            text: "󰅂"
            color: Tokens.color("popup", "accent")

            MouseArea {
                anchors.fill: parent
                onClicked: root.shift(1)
            }
        }
    }

    DayOfWeekRow {
        Layout.fillWidth: true

        delegate: PanelLabel {
            required property var model

            horizontalAlignment: Text.AlignHCenter
            text: model.shortName
            color: Tokens.color("popup", "muted")
        }
    }

    MonthGrid {
        Layout.fillWidth: true
        month: root.shown.getMonth()
        year: root.shown.getFullYear()

        delegate: PanelLabel {
            required property var model

            horizontalAlignment: Text.AlignHCenter
            text: model.day
            opacity: model.month === root.shown.getMonth() ? 1 : 0.35
            color: model.today
                ? Tokens.color("popup", "accent")
                : Tokens.color("popup", "foreground")
            font.bold: model.today
        }
    }
}

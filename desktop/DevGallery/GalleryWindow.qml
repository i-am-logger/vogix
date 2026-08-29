pragma ComponentBehavior: Bound
// The dev gallery: every surface's tokens rendered as swatches plus the
// shell's shared text styles — the visual smoke target ("does the theme
// actually reach every surface"). Opened via `vogix desktop gallery`.
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Wayland
import qs.Services
import qs.Vogix

PanelWindow {
    id: root

    property bool open: false

    visible: open
    anchors {}
    implicitWidth: 660
    implicitHeight: 520
    color: "transparent"
    exclusionMode: ExclusionMode.Ignore
    WlrLayershell.layer: WlrLayer.Overlay
    WlrLayershell.keyboardFocus: visible
        ? WlrKeyboardFocus.OnDemand
        : WlrKeyboardFocus.None

    Rectangle {
        anchors.fill: parent
        radius: 10
        color: Tokens.color("popup", "background")
        border.width: 1
        border.color: Tokens.color("popup", "border")

        Keys.onEscapePressed: root.open = false

        ColumnLayout {
            anchors {
                fill: parent
                margins: 16
            }
            spacing: 10

            Text {
                text: "vogix gallery — " + (Theme.doc.theme ?? "?") + " / " + (Theme.doc.variant ?? "?")
                color: Tokens.color("popup", "accent")
                font.family: Config.fontFamily
                font.pixelSize: Config.fontSize + 3
                font.bold: true
            }

            Flickable {
                Layout.fillWidth: true
                Layout.fillHeight: true
                contentHeight: rows.implicitHeight
                clip: true

                ColumnLayout {
                    id: rows
                    width: parent.width
                    spacing: 8

                    Repeater {
                        model: Object.keys(Config.doc.surfaces ?? {}).sort()

                        ColumnLayout {
                            id: surfaceRow

                            required property string modelData

                            Layout.fillWidth: true
                            spacing: 4

                            Text {
                                text: surfaceRow.modelData
                                color: Tokens.color("popup", "foreground")
                                font.family: Config.fontFamily
                                font.pixelSize: Config.fontSize
                                font.bold: true
                            }

                            RowLayout {
                                Layout.fillWidth: true
                                spacing: 6

                                Repeater {
                                    model: Object.keys((Config.doc.surfaces ?? {})[surfaceRow.modelData] ?? {}).sort()

                                    ColumnLayout {
                                        id: tokenCol

                                        required property string modelData

                                        spacing: 2

                                        Rectangle {
                                            implicitWidth: 56
                                            implicitHeight: 24
                                            radius: 4
                                            color: Tokens.color(surfaceRow.modelData, tokenCol.modelData)
                                            border.width: 1
                                            border.color: Tokens.color("popup", "border")
                                        }

                                        Text {
                                            text: tokenCol.modelData
                                            color: Tokens.color("popup", "muted")
                                            font.family: Config.fontFamily
                                            font.pixelSize: Config.fontSize - 4
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

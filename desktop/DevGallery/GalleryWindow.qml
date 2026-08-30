pragma ComponentBehavior: Bound
// The dev gallery: every surface's tokens rendered as swatches plus the
// shell's shared text styles — the visual smoke target ("does the theme
// actually reach every surface"). Opened via `vogix desktop gallery`.
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Wayland
import qs.Components
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
                font.pixelSize: Metrics.heading
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
                                font.pixelSize: Metrics.body
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
                                            font.pixelSize: Metrics.caption
                                        }
                                    }
                                }
                            }
                        }
                    }

                    Text {
                        text: "components"
                        color: Tokens.color("popup", "foreground")
                        font.family: Config.fontFamily
                        font.pixelSize: Metrics.body
                        font.bold: true
                    }

                    // Live instances of every qs.Components primitive, in
                    // semantic colors — rendering here without warnings is
                    // the components' smoke test.
                    RowLayout {
                        id: demo

                        readonly property color ok: Theme.semantic.success ?? "#ff00ff"
                        readonly property color warn: Theme.semantic.warning ?? "#ff00ff"
                        readonly property color bad: Theme.semantic.danger ?? "#ff00ff"
                        readonly property color dim: Theme.semantic.foreground_comment ?? "#ff00ff"
                        readonly property color text: Theme.semantic.foreground_text ?? "#ff00ff"
                        readonly property color faint: Qt.alpha(Theme.semantic.foreground_border ?? "#ff00ff", 0.22)
                        readonly property color hairline: Qt.alpha(Theme.semantic.foreground_border ?? "#ff00ff", 0.35)

                        Layout.fillWidth: true
                        spacing: 12

                        HudPanel {
                            title: "VU"
                            borderColor: demo.hairline
                            titleColor: demo.dim
                            chipColor: Tokens.color("popup", "background")

                            Column {
                                spacing: 4

                                VuMeter {
                                    label: "OUT"
                                    value: 0.62
                                    peak: 0.8
                                    db: -9.5
                                    low: demo.ok
                                    mid: demo.warn
                                    high: demo.bad
                                    unlit: demo.faint
                                    capColor: demo.dim
                                    labelColor: demo.dim
                                    valueColor: demo.text
                                }

                                VuMeter {
                                    label: "MIC"
                                    value: 0.3
                                    peak: 0.45
                                    db: -21.0
                                    low: demo.ok
                                    mid: demo.warn
                                    high: demo.bad
                                    unlit: demo.faint
                                    capColor: demo.dim
                                    labelColor: demo.dim
                                    valueColor: demo.text
                                }
                            }
                        }

                        HudPanel {
                            title: "SYS"
                            borderColor: demo.hairline
                            titleColor: demo.dim
                            chipColor: Tokens.color("popup", "background")

                            Column {
                                spacing: 4

                                Row {
                                    spacing: 6

                                    HudLabel {
                                        text: "CPU"
                                        color: demo.dim
                                        anchors.verticalCenter: parent.verticalCenter
                                    }

                                    SegmentedMeter {
                                        width: 96
                                        height: 10
                                        value: 0.45
                                        peak: 0.6
                                        low: demo.ok
                                        mid: demo.warn
                                        high: demo.bad
                                        unlit: demo.faint
                                        capColor: demo.dim
                                        anchors.verticalCenter: parent.verticalCenter
                                    }

                                    NumericText {
                                        text: "45%"
                                        widestText: "100%"
                                        color: demo.text
                                        anchors.verticalCenter: parent.verticalCenter
                                    }
                                }

                                Sparkline {
                                    width: 160
                                    height: 28
                                    values: [0.2, 0.35, 0.3, 0.5, 0.45, 0.7, 0.6, 0.8, 0.65, 0.5, 0.55, 0.4]
                                    secondary: [0.1, 0.15, 0.2, 0.15, 0.3, 0.25, 0.4, 0.3, 0.35, 0.25, 0.2, 0.3]
                                    lineColor: Theme.semantic.active ?? "#ff00ff"
                                    secondaryColor: demo.dim
                                }
                            }
                        }

                        Item {
                            implicitWidth: alertLabel.width + 24
                            implicitHeight: 40

                            DashedBorder {
                                color: demo.bad
                            }

                            HudLabel {
                                id: alertLabel
                                anchors.centerIn: parent
                                text: "ALERT"
                                color: demo.bad
                            }
                        }
                    }
                }
            }
        }
    }
}

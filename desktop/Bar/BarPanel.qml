pragma ComponentBehavior: Bound
// One bar on one screen edge. Horizontal bars anchor left+right+edge,
// vertical ones top+bottom+edge (the layer-shell 1-or-3 anchor rule);
// the free axis takes the implicit size, so `thickness` is the whole
// geometry. Hiding PARKS the layer past its edge instead of unmapping
// (remapping costs ~150 ms, a slide ~20 ms and keeps the widgets warm).
// The center section is anchored to the true center — never pushed
// around by how wide start/end happen to be.
import QtQuick
import Quickshell
import qs.Bar.widgets
import qs.Components
import qs.Services
import qs.Vogix

PanelWindow {
    id: panel

    required property string edge
    readonly property bool vertical: edge === "left" || edge === "right"
    readonly property var conf: (Config.bars ?? {})[edge] ?? ({})
    readonly property var confLayout: conf.layout ?? ({})
    readonly property int thickness: conf.size ?? 32
    readonly property bool parked: BarState.isHidden(edge)
    // Breathing room between the bar's outer edge and its first widget.
    readonly property int inset: vertical ? Metrics.unit * 2 : Metrics.unit * 4

    visible: conf.enable ?? false

    anchors {
        top: edge === "top" || vertical
        bottom: edge === "bottom" || vertical
        left: edge === "left" || !vertical
        right: edge === "right" || !vertical
    }

    // Only the unanchored axis consumes the implicit size.
    implicitWidth: thickness
    implicitHeight: thickness

    exclusiveZone: parked ? 0 : thickness
    margins.top: edge === "top" && parked ? -thickness : 0
    margins.bottom: edge === "bottom" && parked ? -thickness : 0
    margins.left: edge === "left" && parked ? -thickness : 0
    margins.right: edge === "right" && parked ? -thickness : 0
    color: "transparent"

    Rectangle {
        anchors.fill: parent
        color: Tokens.color("bar", "background")

        // The chrome line: a full-strength hairline on the bar's inner
        // (content-facing) edge — what makes the bar read as INSTRUMENT
        // PANEL rather than tinted wallpaper.
        Rectangle {
            color: Tokens.color("bar", "border")
            anchors.left: panel.edge === "right" ? parent.left : undefined
            anchors.right: panel.edge === "left" ? parent.right : undefined
            anchors.top: panel.edge === "bottom" ? parent.top : undefined
            anchors.bottom: panel.edge === "top" ? parent.bottom : undefined
            width: panel.vertical ? 1 : parent.width
            height: panel.vertical ? parent.height : 1
        }

        ScanlineOverlay {}

        BarAxis {
            id: axisCtx
            vertical: panel.vertical
            thickness: panel.thickness
            edge: panel.edge
        }

        Section {
            names: panel.confLayout.start ?? []
            axis: axisCtx
            anchors.left: panel.vertical ? undefined : parent.left
            anchors.leftMargin: panel.vertical ? 0 : panel.inset
            anchors.verticalCenter: panel.vertical ? undefined : parent.verticalCenter
            anchors.top: panel.vertical ? parent.top : undefined
            anchors.topMargin: panel.vertical ? panel.inset : 0
            anchors.horizontalCenter: panel.vertical ? parent.horizontalCenter : undefined
        }

        Section {
            names: panel.confLayout.center ?? []
            axis: axisCtx
            anchors.centerIn: parent
        }

        Section {
            names: panel.confLayout.end ?? []
            axis: axisCtx
            anchors.right: panel.vertical ? undefined : parent.right
            anchors.rightMargin: panel.vertical ? 0 : panel.inset
            anchors.verticalCenter: panel.vertical ? undefined : parent.verticalCenter
            anchors.bottom: panel.vertical ? parent.bottom : undefined
            anchors.bottomMargin: panel.vertical ? panel.inset : 0
            anchors.horizontalCenter: panel.vertical ? parent.horizontalCenter : undefined
        }
    }
}

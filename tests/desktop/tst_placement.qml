// Placement: the panel popup and the notification column never cover a
// bar, and a popup summoned from a bar widget opens beside that bar,
// centred on the widget. Geometry is the shipped default HUD on a
// 3840×2160 screen: top 96, bottom 96, left 128, right 114.
import QtQuick
import QtTest
import qs.Geometry

TestCase {
    id: tc

    name: "Placement"

    readonly property size screen: Qt.size(3840, 2160)
    // Bar thicknesses (Item already owns top/bottom/left/right).
    readonly property int topBar: 96
    readonly property int bottomBar: 96
    readonly property int leftRail: 128
    readonly property int rightRail: 114
    readonly property rect area: Placement.freeArea(screen, topBar, bottomBar, leftRail, rightRail)
    // PanelPopup's full height.
    readonly property size panel: Qt.size(380, 560)

    function comparePoint(actual: point, x: real, y: real, what: string): void {
        compare(actual.x, x, what + " x");
        compare(actual.y, y, what + " y");
    }

    // A widget's rect on screen, from its rect inside the `edge` bar.
    function onBar(edge: string, thickness: int, x: real, y: real, w: real, h: real): rect {
        const o = Placement.barOrigin(edge, thickness, tc.screen, tc.topBar);
        return Qt.rect(o.x + x, o.y + y, w, h);
    }

    function test_freeAreaClearsEveryBar(): void {
        compare(area.x, leftRail + Placement.gap);
        compare(area.y, topBar + Placement.gap);
        compare(area.x + area.width, screen.width - rightRail - Placement.gap);
        compare(area.y + area.height, screen.height - bottomBar - Placement.gap);
    }

    function test_parkedBarFreesItsEdge(): void {
        const a = Placement.freeArea(screen, 0, bottomBar, leftRail, rightRail);
        compare(a.y, Placement.gap);
        compare(a.y + a.height, screen.height - bottomBar - Placement.gap);
    }

    function test_barOrigins(): void {
        comparePoint(Placement.barOrigin("top", topBar, screen, topBar), 0, 0, "top bar");
        comparePoint(Placement.barOrigin("bottom", bottomBar, screen, topBar), 0, screen.height - bottomBar, "bottom bar");
        comparePoint(Placement.barOrigin("left", leftRail, screen, topBar), 0, topBar, "left rail");
        comparePoint(Placement.barOrigin("right", rightRail, screen, topBar), screen.width - rightRail, topBar, "right rail");
    }

    // The notification column and a verb-opened panel: under the top
    // bar's end, clear of the right rail.
    function test_unanchoredTakesTheTopRightOfTheFreeArea(): void {
        const column = Qt.size(440, 900);
        const o = Placement.popupOrigin("", Qt.rect(0, 0, 0, 0), column, area);
        comparePoint(o, screen.width - rightRail - Placement.gap - column.width, topBar + Placement.gap, "column");
        verify(o.y >= topBar + Placement.gap, "below the top bar");
        verify(o.x + column.width <= screen.width - rightRail - Placement.gap, "clear of the right rail");
    }

    function test_leftRailPopupOpensBesideTheRail(): void {
        const widget = onBar("left", leftRail, 10, 900, 108, 120);
        const o = Placement.popupOrigin("left", widget, panel, area);
        comparePoint(o, leftRail + Placement.gap, widget.y + (widget.height - panel.height) / 2, "left-rail popup");
    }

    function test_rightRailPopupOpensBesideTheRail(): void {
        const widget = onBar("right", rightRail, 4, 700, 106, 90);
        const o = Placement.popupOrigin("right", widget, panel, area);
        comparePoint(o, screen.width - rightRail - Placement.gap - panel.width,
            widget.y + (widget.height - panel.height) / 2, "right-rail popup");
    }

    function test_railPopupClampsIntoTheFreeArea(): void {
        const high = Placement.popupOrigin("left", onBar("left", leftRail, 10, 4, 108, 60), panel, area);
        compare(high.y, topBar + Placement.gap, "a widget at the rail's top end");
        const low = Placement.popupOrigin("left", onBar("left", leftRail, 10, 1900, 108, 60), panel, area);
        compare(low.y + panel.height, screen.height - bottomBar - Placement.gap, "a widget at the rail's bottom end");
    }

    function test_topBarPopupDropsUnderTheWidget(): void {
        const widget = onBar("top", topBar, 1800, 10, 100, 76);
        const o = Placement.popupOrigin("top", widget, panel, area);
        comparePoint(o, widget.x + (widget.width - panel.width) / 2, topBar + Placement.gap, "top-bar popup");
    }

    function test_topBarEndPopupStaysOnScreen(): void {
        const clock = onBar("top", topBar, 3600, 10, 230, 76);
        const o = Placement.popupOrigin("top", clock, panel, area);
        compare(o.x + panel.width, screen.width - rightRail - Placement.gap);
    }

    function test_bottomBarPopupRisesAboveTheBar(): void {
        const widget = onBar("bottom", bottomBar, 300, 10, 150, 76);
        const o = Placement.popupOrigin("bottom", widget, panel, area);
        comparePoint(o, widget.x + (widget.width - panel.width) / 2,
            screen.height - bottomBar - Placement.gap - panel.height, "bottom-bar popup");
    }

    function test_oversizedSurfaceKeepsItsTopLeftInside(): void {
        // Free area 242×92: the 380×560 panel overflows it both ways.
        const small = Placement.freeArea(Qt.size(500, 300), topBar, bottomBar, leftRail, rightRail);
        const o = Placement.popupOrigin("right", Qt.rect(400, 150, 50, 50), panel, small);
        comparePoint(o, small.x, small.y, "oversized");
    }

    function test_originsAreWholePixels(): void {
        const o = Placement.popupOrigin("left", Qt.rect(0, 1000.3, 10, 33), Qt.size(380, 301.5), area);
        compare(o.y, Math.round(o.y));
    }
}

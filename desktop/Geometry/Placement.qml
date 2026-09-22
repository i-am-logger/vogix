pragma Singleton
// Where the HUD's floating surfaces sit on a screen. The panel popup and
// the notification column use ExclusionMode.Ignore, so they are placed
// from the raw screen edge and every bar's live thickness arrives here as
// an inset. Pure geometry over its arguments — no services, no
// quickshell types — so the shell and the desktop-logic check run the
// same rule.
import QtQuick

QtObject {
    id: root

    // Clearance between a bar's inner edge and a surface beside it.
    readonly property int gap: 8

    // The part of a `screen`-sized output the bars leave free, inset by
    // `gap` on every side. A thickness is 0 for an off or parked bar.
    function freeArea(screen: size, top: int, bottom: int, left: int, right: int): rect {
        return Qt.rect(left + root.gap, top + root.gap,
            Math.max(0, screen.width - left - right - 2 * root.gap),
            Math.max(0, screen.height - top - bottom - 2 * root.gap));
    }

    // Where the `edge` bar's window starts on its screen. The horizontal
    // bars span the full width (Bar.qml maps them before the rails); the
    // rails start below the top bar's exclusive zone, `topZone`.
    function barOrigin(edge: string, thickness: int, screen: size, topZone: int): point {
        switch (edge) {
        case "bottom":
            return Qt.point(0, screen.height - thickness);
        case "left":
            return Qt.point(0, topZone);
        case "right":
            return Qt.point(screen.width - thickness, topZone);
        default:
            return Qt.point(0, 0);
        }
    }

    // Top-left of a `popup`-sized surface summoned from the widget at
    // `anchor` (screen coordinates) on the `edge` bar: beside that bar,
    // centred on the widget along the bar's axis, clamped into `area`.
    // Edge "" (summoned by a verb, no widget): the area's top-right
    // corner, under the top bar's end. A surface larger than the area
    // keeps its top-left inside it. Whole pixels: layer-shell margins are
    // integers.
    function popupOrigin(edge: string, anchor: rect, popup: size, area: rect): point {
        const farX = Math.max(area.x, area.x + area.width - popup.width);
        const farY = Math.max(area.y, area.y + area.height - popup.height);
        const alongX = root.clamp(anchor.x + (anchor.width - popup.width) / 2, area.x, farX);
        const alongY = root.clamp(anchor.y + (anchor.height - popup.height) / 2, area.y, farY);
        switch (edge) {
        case "top":
            return Qt.point(Math.round(alongX), Math.round(area.y));
        case "bottom":
            return Qt.point(Math.round(alongX), Math.round(farY));
        case "left":
            return Qt.point(Math.round(area.x), Math.round(alongY));
        case "right":
            return Qt.point(Math.round(farX), Math.round(alongY));
        default:
            return Qt.point(Math.round(farX), Math.round(area.y));
        }
    }

    function clamp(v: real, lo: real, hi: real): real {
        return Math.max(lo, Math.min(v, hi));
    }
}

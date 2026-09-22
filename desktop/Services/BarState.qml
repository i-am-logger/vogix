pragma Singleton
// Per-edge bar visibility — the ONE mutation point. Bars park (slide
// off-screen) rather than unmap; everything bar-adjacent (panel popup,
// OSD) reads effective thickness from here so it follows hides live, and
// the data sources behind the widgets read `live` so a bar nobody can see
// samples nothing.
import QtQuick
import Quickshell
import qs.Vogix

Singleton {
    id: root

    readonly property list<string> edges: ["top", "bottom", "left", "right"]

    // Whole-object reassigned on every change so bindings re-evaluate.
    property var hiddenEdges: ({ top: false, bottom: false, left: false, right: false })

    function isHidden(edge: string): bool {
        return hiddenEdges[edge] ?? false;
    }

    // Whether anything a bar draws can be seen at all: not while the
    // session lock covers every output, the screensaver covers them, or the
    // idle stage has switched them off.
    readonly property bool viewable: !Lock.locked && !Idle.screensaverActive && !Idle.screensOff

    // An edge's widgets are on screen: the bar is enabled, not parked, and
    // the screen is viewable. Parked widgets stay instantiated (warm) but
    // their taps and samplers stop.
    function live(edge: string): bool {
        return enabled(edge) && !isHidden(edge) && root.viewable;
    }

    function enabled(edge: string): bool {
        return ((Config.bars ?? {})[edge] ?? {}).enable ?? false;
    }

    // What the edge takes from the screen right now: 0 when off or parked.
    function thickness(edge: string): int {
        if (!enabled(edge) || isHidden(edge))
            return 0;
        return ((Config.bars ?? {})[edge] ?? {}).size ?? 0;
    }

    function setHidden(edge: string, value: bool): string {
        const targets = (edge === "all" || edge === "") ? edges : [edge];
        if (!targets.every(e => edges.includes(e)))
            return "unknown edge: " + edge;
        const next = Object.assign({}, hiddenEdges);
        for (const e of targets)
            next[e] = value;
        hiddenEdges = next;
        return statusLine();
    }

    function toggleEdge(edge: string): string {
        if (edge === "all" || edge === "")
            return setHidden("all", !isHidden("top"));
        if (!edges.includes(edge))
            return "unknown edge: " + edge;
        return setHidden(edge, !isHidden(edge));
    }

    function statusLine(): string {
        return edges
            .map(e => e + ":" + (!enabled(e) ? "off" : (isHidden(e) ? "hidden" : "shown")))
            .join(" ");
    }

    // Every widget slot the bars have placed (Section adds and drops its
    // Loaders; each carries its bar's BarAxis as `axis` and its name as
    // `modelData`). Whole-array reassigned, like hiddenEdges.
    property var slots: []

    function place(slot: var): void {
        root.slots = root.slots.concat([slot]);
    }

    function unplace(slot: var): void {
        root.slots = root.slots.filter(s => s !== slot);
    }

    // One line per placed widget, in placement order: the screen, the bar
    // edge, the widget name, and where it sits on that screen when its bar
    // is shown, as "x y width height" in logical pixels.
    function geometry(): string {
        return root.slots
            .filter(s => s.axis?.window)
            .map(s => {
                const r = s.axis.screenRect(s);
                return [s.axis.window.screen?.name ?? "-", s.axis.edge, s.modelData,
                    Math.round(r.x), Math.round(r.y), Math.round(r.width), Math.round(r.height)].join(" ");
            })
            .join("\n");
    }
}

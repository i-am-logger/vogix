pragma Singleton
// Per-edge bar visibility — the ONE mutation point. Bars park (slide
// off-screen) rather than unmap; everything bar-adjacent (panel popup,
// OSD) reads effective thickness from here so it follows hides live.
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
}

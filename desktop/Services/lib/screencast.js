.pragma library
// Hyprland screencast events → the number of live capture sessions.
// Pure; Privacy.qml feeds it the socket2 events.

// Applies one `screencast` event payload ("<state>,<kind>": state 1 when a
// session starts delivering frames, 0 when it stops; kind is "monitor",
// "window" or "region", numeric on older Hyprland) to the live count.
// Each session posts exactly one start and one stop, so the count is
// their difference. A stop for a session that started before the shell
// did would take it below zero; it floors at zero instead.
function step(count, data) {
    const state = data.split(",")[0];
    if (state === "1")
        return count + 1;
    if (state === "0")
        return Math.max(0, count - 1);
    return count;
}

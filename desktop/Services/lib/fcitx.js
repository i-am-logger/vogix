.pragma library
// fcitx5's org.fcitx.Fcitx.Controller1 replies, as dbus-send --print-reply
// prints them, and what the language chip shows and does with them. Pure
// functions; Services/Fcitx.qml runs the tools.

// The quoted strings of a reply, in order.
function strings(lines) {
    const out = [];
    for (const line of lines) {
        const m = /^\s*string "(.*)"$/.exec(line);
        if (m)
            out.push(m[1]);
    }
    return out;
}

// CurrentInputMethod() -> s: the input method in use.
function currentIM(lines) {
    return strings(lines)[0] || "";
}

// InputMethodGroupInfo(s) -> sa(ss): the group's default layout, then each
// input method's name and its layout, in pairs.
function groupInfo(lines) {
    const s = strings(lines);
    const ims = [];
    for (let i = 1; i < s.length; i += 2)
        ims.push(s[i]);
    return { layout: s[0] || "", ims: ims };
}

// The input methods that compose: every one but a keyboard layout's.
function engines(ims) {
    return ims.filter(name => !name.startsWith("keyboard-"));
}

// What the chip shows: an engine's name while it is the input method in
// use, else the xkb layout, as without fcitx5.
function chip(current, engines, xkb) {
    return current && engines.includes(current) ? current : xkb;
}

// What a click does: toggle fcitx5 between the keyboard and the engine when
// the group has one, else switch the xkb layout as without fcitx5.
function click(engines) {
    return engines.length > 0 ? "toggle" : "xkb-next";
}

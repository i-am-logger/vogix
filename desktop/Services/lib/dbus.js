.pragma library
// dbus-monitor and dbus-send text output → the facts the shell acts on.
// Pure functions; the QML side runs the tools.

// The match rule that delivers exactly the ownership changes of one
// well-known name on the bus.
function ownerChangeRule(name) {
    return "type='signal',sender='org.freedesktop.DBus',interface='org.freedesktop.DBus',"
        + "member='NameOwnerChanged',arg0='" + name + "'";
}

// A fresh parser state for step().
function initialState() {
    return { args: null };
}

// Feeds one dbus-monitor output line. Returns { state, event } where event
// is null or one of:
//   { kind: "subscribed" } — the monitor's own NameAcquired. libdbus
//       dispatches it only after the AddMatch call has returned, so from
//       here on no ownership change can slip past the monitor.
//   { kind: "owner", name, oldOwner, newOwner } — a NameOwnerChanged,
//       once its three string arguments have been read.
function step(state, line) {
    if (/^(signal|method call|method return|error) /.test(line)) {
        if (/ member=NameAcquired$/.test(line))
            return { state: { args: null }, event: { kind: "subscribed" } };
        const opens = / member=NameOwnerChanged$/.test(line);
        return { state: { args: opens ? [] : null }, event: null };
    }
    if (state.args === null)
        return { state: state, event: null };
    const m = line.match(/^\s+string "(.*)"$/);
    if (!m)
        return { state: { args: null }, event: null };
    const args = state.args.concat([m[1]]);
    if (args.length < 3)
        return { state: { args: args }, event: null };
    return {
        state: { args: null },
        event: { kind: "owner", name: args[0], oldOwner: args[1], newOwner: args[2] },
    };
}

// `dbus-send --print-reply ... NameHasOwner` output → whether the name
// has an owner.
function hasOwnerReply(text) {
    return /^\s*boolean true\s*$/m.test(text);
}

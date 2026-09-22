.pragma library
// Tailnet state for Services/Tailscale.qml: the link state, the peers,
// and how long the current connection has lasted. Pure functions; the
// QML side runs data/tailscale-status.sh and keeps the record.

// What the node's tailnet link is doing.
var Link = Object.freeze({
    // No tailscale CLI on this host.
    Absent: 0,
    // The CLI cannot reach tailscaled.
    DaemonDown: 1,
    // `tailscale down`.
    Stopped: 2,
    // Needs a login or machine approval.
    LoggedOut: 3,
    // Starting up.
    Connecting: 4,
    // Up, but the control plane does not see the node online.
    Offline: 5,
    Connected: 6,
});

// tailscale-status.sh output → { daemonStartMs, link, selfName, selfIp,
// tailnet, peers: [{ name, ip, online }] online-first, peersOnline }.
function parseReport(text) {
    const cut = text.indexOf("\n");
    const head = cut < 0 ? text : text.slice(0, cut);
    const body = (cut < 0 ? "" : text.slice(cut + 1)).trim();
    const m = head.trim().match(/^@(\d+)$/);
    const report = {
        daemonStartMs: m && Number(m[1]) > 0 ? Number(m[1]) * 1000 : 0,
        link: Link.DaemonDown,
        selfName: "",
        selfIp: "",
        tailnet: "",
        peers: [],
        peersOnline: 0,
    };
    if (body === "absent") {
        report.link = Link.Absent;
        return report;
    }
    let doc;
    try {
        doc = JSON.parse(body);
    } catch (e) {
        return report;
    }
    report.link = linkOf(doc);
    const host = h => (h ?? "").replace(/\.$/, "");
    report.selfName = host(doc.Self?.DNSName);
    report.selfIp = (doc.TailscaleIPs ?? [])[0] ?? "";
    report.tailnet = doc.CurrentTailnet?.Name ?? "";
    report.peers = Object.values(doc.Peer ?? {}).map(p => ({
        name: host(p.DNSName).split(".")[0],
        ip: (p.TailscaleIPs ?? [])[0] ?? "",
        online: p.Online === true,
    })).sort((a, b) => (b.online - a.online) || a.name.localeCompare(b.name));
    report.peersOnline = report.peers.filter(p => p.online).length;
    return report;
}

function linkOf(doc) {
    switch (doc.BackendState) {
    case "Running":
        return doc.Self?.Online === true ? Link.Connected : Link.Offline;
    case "Stopped":
        return Link.Stopped;
    case "NeedsLogin":
    case "NeedsMachineAuth":
        return Link.LoggedOut;
    default:
        return Link.Connecting;
    }
}

// The connection record after one observation: { since, exact,
// daemonStartMs }, since = 0 while not connected. `prev` is the record
// so far, which on a fresh shell is the one the previous shell left in
// the runtime directory; `first` marks the shell's first observation.
//
// A connection carries over while nothing says it broke: the same
// tailscaled run (a restarted daemon dropped every connection), and on a
// fresh shell that run check is required, since the shell cannot vouch
// for the time it was not watching. A connection the shell sees begin
// dates from that observation and is exact to the poll period; one it
// only finds already up dates from the finding, a lower bound.
function track(prev, report, nowMs, first) {
    const daemon = report.daemonStartMs;
    if (report.link !== Link.Connected)
        return { since: 0, exact: false, daemonStartMs: daemon };
    const sameDaemon = daemon > 0 && prev.daemonStartMs === daemon;
    const carries = prev.since > 0 && (sameDaemon || (!first && daemon === 0));
    if (carries)
        return { since: prev.since, exact: prev.exact, daemonStartMs: daemon };
    return { since: nowMs, exact: !first && prev.since === 0, daemonStartMs: daemon };
}

// "12m" / "3h5m" / "2d"; "≥" marks a lower bound; "" when not connected.
function sinceText(record, nowMs) {
    if (record.since <= 0)
        return "";
    const mins = Math.max(0, Math.floor((nowMs - record.since) / 60000));
    let t;
    if (mins < 60)
        t = mins + "m";
    else if (mins < 60 * 24)
        t = Math.floor(mins / 60) + "h" + (mins % 60 > 0 ? (mins % 60) + "m" : "");
    else
        t = Math.floor(mins / (60 * 24)) + "d";
    return (record.exact ? "" : "≥") + t;
}

// A persisted record (the runtime file's text) → a record, or a
// not-connected one when absent or unreadable.
function parseRecord(text) {
    try {
        const r = JSON.parse(text);
        if (typeof r.since === "number" && typeof r.exact === "boolean" && typeof r.daemonStartMs === "number")
            return { since: r.since, exact: r.exact, daemonStartMs: r.daemonStartMs };
    } catch (e) {}
    return { since: 0, exact: false, daemonStartMs: 0 };
}

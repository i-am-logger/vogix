import QtQuick
import QtTest
import "../../desktop/Services/lib/tailnet.js" as Tailnet

TestCase {
    name: "Tailnet"

    readonly property real daemonA: 1790100514000
    readonly property real daemonB: 1790200000000
    readonly property real t0: 1790150000000
    readonly property var none: ({ since: 0, exact: false, daemonStartMs: 0 })

    function status(backend, online, peers) {
        return JSON.stringify({
            BackendState: backend,
            TailscaleIPs: ["100.97.96.49"],
            CurrentTailnet: { Name: "example.ts.net" },
            Self: { DNSName: "yoga.tail46cce1.ts.net.", Online: online },
            Peer: peers ?? {},
        });
    }

    function report(start, body) {
        return Tailnet.parseReport(start + "\n" + body);
    }

    function connected(start) {
        return report(start, status("Running", true));
    }

    function test_report_fields() {
        const r = report("@1790100514", status("Running", true, {
            a: { DNSName: "zeta.tail46cce1.ts.net.", TailscaleIPs: ["100.1.1.1"], Online: false },
            b: { DNSName: "alpha.tail46cce1.ts.net.", TailscaleIPs: ["100.2.2.2"], Online: true },
            c: { DNSName: "beta.tail46cce1.ts.net.", TailscaleIPs: ["100.3.3.3"], Online: true },
        }));
        compare(r.daemonStartMs, daemonA);
        compare(r.link, Tailnet.Link.Connected);
        compare(r.selfName, "yoga.tail46cce1.ts.net");
        compare(r.selfIp, "100.97.96.49");
        compare(r.tailnet, "example.ts.net");
        compare(r.peers.map(p => p.name), ["alpha", "beta", "zeta"]);
        compare(r.peersOnline, 2);
    }

    function test_link_states() {
        compare(report("", "absent").link, Tailnet.Link.Absent);
        compare(report("@1", "down").link, Tailnet.Link.DaemonDown);
        compare(report("@1", "{ truncated").link, Tailnet.Link.DaemonDown);
        compare(report("@1", status("Stopped", false)).link, Tailnet.Link.Stopped);
        compare(report("@1", status("NeedsLogin", false)).link, Tailnet.Link.LoggedOut);
        compare(report("@1", status("NeedsMachineAuth", false)).link, Tailnet.Link.LoggedOut);
        compare(report("@1", status("Starting", false)).link, Tailnet.Link.Connecting);
        compare(report("@1", status("Running", false)).link, Tailnet.Link.Offline);
        compare(report("", "absent").daemonStartMs, 0);
    }

    // The shell sees the link come up: the clock starts then, exactly.
    function test_observed_connection_is_exact() {
        let rec = Tailnet.track(none, report("@1790100514", status("Stopped", false)), t0, true);
        compare(rec.since, 0);
        rec = Tailnet.track(rec, connected("@1790100514"), t0 + 30000, false);
        compare(rec.since, t0 + 30000);
        compare(rec.exact, true);
        rec = Tailnet.track(rec, connected("@1790100514"), t0 + 3600000, false);
        compare(rec.since, t0 + 30000);
    }

    // The daemon's uptime is not the connection's: found already up with
    // no record, the connection dates from the finding, as a lower bound.
    function test_found_connection_is_a_lower_bound() {
        const rec = Tailnet.track(none, connected("@1790100514"), t0, true);
        compare(rec.since, t0);
        compare(rec.exact, false);
        verify(rec.since !== daemonA);
    }

    // A restarted shell keeps the previous shell's record for the same
    // tailscaled run.
    function test_restart_keeps_the_record_of_the_same_daemon() {
        const kept = { since: t0 - 7200000, exact: true, daemonStartMs: daemonA };
        const rec = Tailnet.track(kept, connected("@1790100514"), t0, true);
        compare(rec.since, t0 - 7200000);
        compare(rec.exact, true);
    }

    // ...but not across a tailscaled restart, which dropped the link.
    function test_restart_drops_the_record_of_another_daemon() {
        const kept = { since: t0 - 7200000, exact: true, daemonStartMs: daemonA };
        const rec = Tailnet.track(kept, connected("@1790200000"), t0, true);
        compare(rec.since, t0);
        compare(rec.exact, false);
    }

    // A daemon restart between two samples breaks the connection too; the
    // reconnect happened somewhere in the gap, so only a lower bound.
    function test_daemon_restart_between_samples() {
        const up = { since: t0 - 600000, exact: true, daemonStartMs: daemonA };
        const rec = Tailnet.track(up, connected("@1790200000"), t0, false);
        compare(rec.since, t0);
        compare(rec.exact, false);
        compare(rec.daemonStartMs, daemonB);
    }

    function test_disconnect_clears_the_clock() {
        const up = { since: t0 - 600000, exact: true, daemonStartMs: daemonA };
        compare(Tailnet.track(up, report("@1790100514", status("Running", false)), t0, false).since, 0);
        compare(Tailnet.track(up, report("@1790100514", "down"), t0, false).since, 0);
    }

    // Without systemd there is no daemon start: a running shell trusts its
    // own continuous observation, a fresh one trusts no record.
    function test_unknown_daemon_start() {
        const up = { since: t0 - 600000, exact: true, daemonStartMs: 0 };
        compare(Tailnet.track(up, connected(""), t0, false).since, t0 - 600000);
        compare(Tailnet.track(up, connected(""), t0, true).since, t0);
    }

    function test_since_text() {
        compare(Tailnet.sinceText(none, t0), "");
        compare(Tailnet.sinceText({ since: t0 - 12 * 60000, exact: true, daemonStartMs: 0 }, t0), "12m");
        compare(Tailnet.sinceText({ since: t0 - 185 * 60000, exact: true, daemonStartMs: 0 }, t0), "3h5m");
        compare(Tailnet.sinceText({ since: t0 - 120 * 60000, exact: false, daemonStartMs: 0 }, t0), "≥2h");
        compare(Tailnet.sinceText({ since: t0 - 3 * 1440 * 60000, exact: true, daemonStartMs: 0 }, t0), "3d");
    }

    function test_record_round_trip() {
        const rec = { since: t0, exact: true, daemonStartMs: daemonA };
        compare(Tailnet.parseRecord(JSON.stringify(rec)), rec);
        compare(Tailnet.parseRecord(""), none);
        compare(Tailnet.parseRecord("{\"since\":\"x\"}"), none);
    }
}

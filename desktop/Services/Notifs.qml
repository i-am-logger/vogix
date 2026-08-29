pragma Singleton
// The shell's notification server — the org.freedesktop.Notifications owner
// (mako's replacement). Rules come from desktop.json: normal popups expire,
// CRITICAL never does, per-app rules pin timeout/accent/DND-bypass.
//
// Live popups are mirrored to ONE state file (atomic setText) so a shell
// restart mid-YubiKey-prompt loses nothing; a capped history feeds
// `vogix desktop notify history` (read directly by the CLI, no IPC needed).
import QtQuick
import Quickshell
import Quickshell.Io
import Quickshell.Services.Notifications
import qs.Vogix

Singleton {
    id: root

    // [{ key, appName, summary, body, urgency, accent, timeout, live }]
    // `live` indexes into liveRefs (server objects are not serializable).
    property var popups: []
    property var liveRefs: ({})
    property bool dnd: false
    property int nextKey: 1

    readonly property var conf: Config.doc.notifications ?? ({})
    readonly property bool enabled: conf.enable ?? true

    function ruleFor(appName) {
        return (conf.appRules ?? {})[appName] ?? {};
    }

    // A popup is visible under DND only when critical or rule-exempt.
    function visibleUnderDnd(p) {
        return !root.dnd || p.urgency === NotificationUrgency.Critical
            || (ruleFor(p.appName).bypassDnd ?? false);
    }

    NotificationServer {
        id: server
        bodySupported: true
        persistenceSupported: true
        keepOnReload: true

        onNotification: n => {
            if (!root.enabled)
                return;
            n.tracked = true;
            const rule = root.ruleFor(n.appName);
            const critical = n.urgency === NotificationUrgency.Critical;
            const entry = {
                key: root.nextKey++,
                appName: n.appName,
                summary: n.summary,
                body: n.body,
                urgency: n.urgency,
                accent: rule.accent ?? null,
                // 0 = never expires. Critical never expires (the mako rule
                // this shell inherits); spec -1 = server default.
                timeout: critical ? 0
                    : (rule.timeout ?? (n.expireTimeout > 0 ? n.expireTimeout
                        : (n.expireTimeout === 0 ? 0 : (root.conf.defaultTimeout ?? 5000)))),
                live: true
            };
            root.liveRefs[entry.key] = n;
            n.closed.connect(() => root.drop(entry.key));
            root.popups = root.popups.concat([entry]);
            root.appendHistory(entry);
            root.save();
        }
    }

    // Remove from the model (server object may already be gone).
    function drop(key) {
        delete root.liveRefs[key];
        root.popups = root.popups.filter(p => p.key !== key);
        root.save();
    }

    function dismiss(key) {
        const n = root.liveRefs[key];
        if (n)
            n.dismiss(); // closed() drops the entry
        else
            root.drop(key); // restored entry: no live object
    }

    function dismissNewest() {
        if (root.popups.length > 0)
            root.dismiss(root.popups[root.popups.length - 1].key);
    }

    function dismissAll() {
        [...root.popups].forEach(p => root.dismiss(p.key));
    }

    function expire(key) {
        const n = root.liveRefs[key];
        if (n)
            n.expire();
        else
            root.drop(key);
    }

    function setDnd(v) {
        root.dnd = v;
        dndFile.setText(JSON.stringify({ dnd: v }));
    }

    // ── persistence ──
    function save() {
        stateFile.setText(JSON.stringify(root.popups.map(p => ({
            key: p.key, appName: p.appName, summary: p.summary,
            body: p.body, urgency: p.urgency, accent: p.accent,
            timeout: p.timeout, live: false
        }))));
    }

    function appendHistory(entry) {
        const h = root.history();
        h.push({
            at: new Date().toISOString(), appName: entry.appName,
            summary: entry.summary, body: entry.body, urgency: entry.urgency
        });
        historyFile.setText(JSON.stringify(h.slice(-100)));
    }

    function history() {
        try {
            return JSON.parse(historyFile.text()) ?? [];
        } catch (e) {
            return [];
        }
    }

    FileView {
        id: stateFile
        path: Paths.stateRoot + "/desktop/notifications.json"
        watchChanges: false
        onLoaded: {
            // Restore what a crash/restart left behind — mid-prompt criticals
            // must survive. Restored entries have no live server object.
            try {
                const kept = JSON.parse(text());
                if (Array.isArray(kept) && kept.length > 0 && root.popups.length === 0) {
                    root.popups = kept;
                    root.nextKey = 1 + kept.reduce((m, p) => Math.max(m, p.key), 0);
                }
            } catch (e) {}
        }
    }

    FileView {
        id: historyFile
        path: Paths.stateRoot + "/desktop/notifications-history.json"
        watchChanges: false
    }

    FileView {
        id: dndFile
        path: Paths.stateRoot + "/desktop/dnd.json"
        watchChanges: false
        onLoaded: {
            try {
                root.dnd = JSON.parse(text()).dnd === true;
            } catch (e) {}
        }
    }
}

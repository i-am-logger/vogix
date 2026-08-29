pragma Singleton
// The launcher: one overlay, many providers — apps (desktop entries), files
// (fd), calc (qalc), emoji (a data file in the package), ssh (~/.ssh/config
// hosts), clipboard (cliphist), the theme and background pickers, the loaded
// root menu, and the dmenu-style select/input sessions behind
// `vogix desktop select|input`. Modes come from desktop.json; the shell
// hardcodes none of the menu content.
import QtQuick
import Quickshell
import Quickshell.Io
import qs.Services
import qs.Vogix

Singleton {
    id: root

    property bool open: false
    property string mode: "apps"
    property string prompt: ""
    property string query: ""
    property var items: []
    property int cursor: 0

    // dmenu session state (select/input verbs).
    property string sessionId: ""
    property var sessionItems: []

    // menu state: the entry list currently shown + which `when` guards passed.
    property var menuEntries: []
    property var guardsPassed: ({})

    // provider caches, loaded on open
    property var emojiRows: []
    property var sshRows: []
    property var clipRows: []
    property var themeRows: []
    property var fileRows: []
    property string calcResult: ""

    readonly property var config: Config.doc.launcher ?? ({})

    function modeEnabled(m: string): bool {
        return ((config.modes ?? {})[m] ?? {}).enable ?? true;
    }

    function status(): string {
        return root.open ? root.mode : "closed";
    }

    function openMode(m: string, q: string): string {
        const wanted = m === "" ? "apps" : m;
        if (wanted !== "menu" && wanted !== "select" && wanted !== "input"
            && !modeEnabled(wanted))
            return "mode disabled: " + wanted;
        root.mode = wanted;
        root.prompt = "";
        root.query = q ?? "";
        root.cursor = 0;
        root.open = true;
        prime();
        refresh();
        return "open: " + wanted;
    }

    function openMenu(summon: string): string {
        const menu = root.config.menu ?? [];
        let entries = menu;
        if (summon !== "") {
            const hit = menu.find(e => e.id === summon);
            if (!hit)
                return "no menu entry: " + summon;
            if ((hit.submenu ?? []).length === 0) {
                if (hit.action)
                    runShell(hit.action);
                return "ran: " + hit.id;
            }
            entries = hit.submenu;
        }
        root.menuEntries = entries;
        root.guardsPassed = {};
        root.mode = "menu";
        root.prompt = "";
        root.query = "";
        root.cursor = 0;
        root.open = true;
        runGuards(entries);
        refresh();
        return "open: menu";
    }

    // A dmenu session: the CLI wrote the items to select-<id>.json and polls
    // select-<id>.result; every exit path (pick, cancel, close) writes the
    // result file so the caller never hangs on us.
    function openSelect(id: string, p: string, textOnly: bool): string {
        root.sessionId = id;
        sessionFile.path = Paths.stateRoot + "/desktop/select-" + id + ".json";
        sessionFile.reload();
        root.mode = textOnly ? "input" : "select";
        root.prompt = p ?? "";
        root.query = "";
        root.cursor = 0;
        root.open = true;
        refresh();
        return "open: " + root.mode;
    }

    function close(): string {
        if (root.sessionId !== "")
            finishSession(null);
        root.open = false;
        root.query = "";
        root.items = [];
        return "closed";
    }

    function toggle(): string {
        return root.open ? close() : openMode("apps", "");
    }

    function setQuery(q: string): void {
        root.query = q;
        root.cursor = 0;
        if (root.mode === "files" || root.mode === "calc")
            debounce.restart();
        else
            refresh();
    }

    function moveCursor(delta: int): void {
        if (root.items.length === 0)
            return;
        root.cursor = Math.min(root.items.length - 1,
                               Math.max(0, root.cursor + delta));
    }

    function activate(index: int): void {
        if (root.mode === "input") {
            finishSession(root.query);
            root.open = false;
            return;
        }
        const row = root.items[index];
        if (!row)
            return;
        switch (root.mode) {
        case "apps":
            row.data.execute();
            break;
        case "files":
            Quickshell.execDetached(["xdg-open", row.data]);
            break;
        case "calc":
        case "emoji":
            copyText(row.data);
            break;
        case "ssh":
            Quickshell.execDetached([
                Quickshell.env("TERMINAL") ?? "alacritty", "-e", "ssh", row.data]);
            break;
        case "clipboard":
            runShell("cliphist decode " + row.data + " | wl-copy");
            break;
        case "theme":
            Quickshell.execDetached(["vogix", "theme", "set", row.data]);
            break;
        case "background":
            Quickshell.execDetached(["vogix", "desktop", "background", "set", row.data]);
            break;
        case "menu": {
            const entry = row.data;
            if ((entry.submenu ?? []).length > 0) {
                root.menuEntries = entry.submenu;
                root.guardsPassed = {};
                root.query = "";
                root.cursor = 0;
                runGuards(entry.submenu);
                refresh();
                return; // stay open, one level down
            }
            if (entry.action)
                runShell(entry.action);
            break;
        }
        case "select":
            finishSession(row.label);
            break;
        }
        root.open = false;
        root.query = "";
        root.items = [];
    }

    // ---- providers ----------------------------------------------------

    function prime(): void {
        switch (root.mode) {
        case "emoji":
            if (root.emojiRows.length === 0)
                emojiFile.reload();
            break;
        case "ssh":
            sshFile.reload();
            break;
        case "clipboard":
            root.clipRows = [];
            clipProc.running = false;
            clipProc.running = true;
            break;
        case "theme":
            if (root.themeRows.length === 0) {
                themeProc.running = false;
                themeProc.running = true;
            }
            break;
        }
    }

    function refresh(): void {
        const q = root.query.toLowerCase();
        switch (root.mode) {
        case "apps": {
            const apps = DesktopEntries.applications.values
                .filter(a => !a.noDisplay)
                .filter(a => q === ""
                    || a.name.toLowerCase().includes(q)
                    || (a.genericName ?? "").toLowerCase().includes(q)
                    || (a.keywords ?? []).some(k => k.toLowerCase().includes(q)))
                .sort((a, b) => a.name.localeCompare(b.name));
            root.items = apps.slice(0, 40).map(a => ({
                icon: "", label: a.name, sublabel: a.genericName ?? "", data: a }));
            break;
        }
        case "files":
            root.items = root.fileRows;
            break;
        case "calc":
            root.items = root.calcResult === "" ? []
                : [{ icon: "", label: root.calcResult,
                     sublabel: "Enter copies to clipboard", data: root.calcResult }];
            break;
        case "emoji":
            root.items = root.emojiRows
                .filter(r => q === "" || r.sublabel.toLowerCase().includes(q))
                .slice(0, 40);
            break;
        case "ssh":
            root.items = root.sshRows
                .filter(r => q === "" || r.label.toLowerCase().includes(q));
            break;
        case "clipboard":
            root.items = root.clipRows
                .filter(r => q === "" || r.label.toLowerCase().includes(q))
                .slice(0, 40);
            break;
        case "theme":
            root.items = root.themeRows
                .filter(r => q === "" || r.label.toLowerCase().includes(q));
            break;
        case "background":
            root.items = Backgrounds.entries.map(e => ({
                icon: "", label: e.name, sublabel: e.kind, data: e.path }));
            break;
        case "menu":
            root.items = root.menuEntries
                .filter(e => !e.when || root.guardsPassed[e.id] === true)
                .filter(e => q === "" || e.label.toLowerCase().includes(q))
                .map(e => ({
                    icon: e.icon ?? "",
                    label: e.label,
                    sublabel: (e.submenu ?? []).length > 0 ? "…" : "",
                    data: e }));
            break;
        case "select":
            root.items = root.sessionItems
                .filter(l => q === "" || l.toLowerCase().includes(q))
                .map(l => ({ icon: "", label: l, sublabel: "", data: l }));
            break;
        case "input":
            root.items = [];
            break;
        }
        root.cursor = Math.min(root.cursor, Math.max(0, root.items.length - 1));
    }

    // Visibility guards: one subprocess per menu open, never one per entry.
    function runGuards(entries: var): void {
        const guarded = entries.filter(e => e.when);
        if (guarded.length === 0)
            return;
        const script = guarded
            .map(e => "if ( " + e.when + " ) >/dev/null 2>&1; then echo 'ok:" + e.id + "'; fi")
            .join("\n");
        guardProc.command = ["sh", "-c", script];
        guardProc.running = false;
        guardProc.running = true;
    }

    function runShell(action: string): void {
        Quickshell.execDetached(["sh", "-c", action]);
    }

    function copyText(value: string): void {
        Quickshell.execDetached(["sh", "-c", "printf %s \"$1\" | wl-copy", "vogix", value]);
    }

    function finishSession(choice: var): void {
        if (root.sessionId === "")
            return;
        resultFile.path = Paths.stateRoot + "/desktop/select-" + root.sessionId + ".result";
        resultFile.setText(JSON.stringify(
            choice === null ? { cancelled: true } : { choice: choice }));
        root.sessionId = "";
        root.sessionItems = [];
    }

    // ---- plumbing -----------------------------------------------------

    Timer {
        id: debounce
        interval: 150
        onTriggered: {
            if (root.mode === "files") {
                if (root.query === "") {
                    root.fileRows = [];
                    root.refresh();
                    return;
                }
                fdProc.command = ["fd", "--type", "f", "--max-results", "40",
                    "--absolute-path", "--full-path", root.query, Paths.home];
                fdProc.running = false;
                fdProc.running = true;
            } else if (root.mode === "calc") {
                if (root.query === "") {
                    root.calcResult = "";
                    root.refresh();
                    return;
                }
                calcProc.command = ["qalc", "-t", root.query];
                calcProc.running = false;
                calcProc.running = true;
            }
        }
    }

    Process {
        id: fdProc
        stdout: StdioCollector {
            onStreamFinished: {
                root.fileRows = text.split("\n").filter(l => l !== "").map(p => ({
                    icon: "",
                    label: p.startsWith(Paths.home) ? "~" + p.slice(Paths.home.length) : p,
                    sublabel: "",
                    data: p }));
                root.refresh();
            }
        }
    }

    Process {
        id: calcProc
        stdout: StdioCollector {
            onStreamFinished: {
                root.calcResult = text.trim();
                root.refresh();
            }
        }
    }

    Process {
        id: clipProc
        command: ["cliphist", "list"]
        stdout: StdioCollector {
            onStreamFinished: {
                root.clipRows = text.split("\n").filter(l => l !== "").map(l => {
                    const tab = l.indexOf("\t");
                    return {
                        icon: "",
                        label: tab > 0 ? l.slice(tab + 1) : l,
                        sublabel: "",
                        data: tab > 0 ? l.slice(0, tab) : l };
                });
                root.refresh();
            }
        }
    }

    Process {
        id: themeProc
        command: ["vogix", "theme", "list"]
        stdout: StdioCollector {
            onStreamFinished: {
                // Parse the human listing: rows after the "Themes…" header,
                // two-space indented, until the blank line before "Total:".
                const rows = [];
                let inThemes = false;
                for (const line of text.split("\n")) {
                    if (line.startsWith("Themes")) {
                        inThemes = true;
                        continue;
                    }
                    if (!inThemes)
                        continue;
                    if (line.trim() === "")
                        break;
                    const name = line.trim().split(" ")[0];
                    if (name !== "")
                        rows.push({ icon: "", label: name, sublabel: "", data: name });
                }
                root.themeRows = rows;
                root.refresh();
            }
        }
    }

    Process {
        id: guardProc
        stdout: StdioCollector {
            onStreamFinished: {
                const passed = {};
                for (const line of text.split("\n"))
                    if (line.startsWith("ok:"))
                        passed[line.slice(3)] = true;
                root.guardsPassed = passed;
                root.refresh();
            }
        }
    }

    FileView {
        id: emojiFile
        path: Quickshell.shellDir + "/data/emoji.txt"
        watchChanges: false
        preload: false
        onLoaded: {
            root.emojiRows = text().split("\n").filter(l => l !== "").map(l => {
                const tab = l.indexOf("\t");
                return {
                    icon: "",
                    label: tab > 0 ? l.slice(0, tab) : l,
                    sublabel: tab > 0 ? l.slice(tab + 1) : "",
                    data: tab > 0 ? l.slice(0, tab) : l };
            });
            root.refresh();
        }
    }

    FileView {
        id: sshFile
        path: Paths.home + "/.ssh/config"
        watchChanges: false
        preload: false
        onLoaded: {
            const rows = [];
            for (const line of text().split("\n")) {
                const m = line.match(/^\s*Host\s+(.+)$/);
                if (!m)
                    continue;
                for (const host of m[1].trim().split(/\s+/))
                    if (!host.includes("*") && !host.includes("?"))
                        rows.push({ icon: "", label: host, sublabel: "ssh", data: host });
            }
            root.sshRows = rows;
            root.refresh();
        }
        onLoadFailed: root.sshRows = []
    }

    FileView {
        id: sessionFile
        watchChanges: false
        preload: false
        onLoaded: {
            try {
                root.sessionItems = JSON.parse(text()).items ?? [];
            } catch (e) {
                root.sessionItems = [];
            }
            root.refresh();
        }
        onLoadFailed: {
            root.sessionItems = [];
            root.refresh();
        }
    }

    FileView {
        id: resultFile
        watchChanges: false
        preload: false
    }
}

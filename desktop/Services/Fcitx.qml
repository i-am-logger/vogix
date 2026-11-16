pragma Singleton
// fcitx5's input method, for the LANG chip. fcitx5 is present while its
// bus name has an owner: asked with NameHasOwner and followed with
// NameOwnerChanged, never by calling it, so asking never starts fcitx5 by
// activation. While present, the chip's state is the current group's input
// methods and the one in use, read with dbus-send through lib/fcitx.js.
// fcitx5 signals a change of its groups (InputMethodGroupsChanged) but not
// of the input method in use, which it keeps per input context: that is
// read again after the chip's own click and on each focus change.
import QtQuick
import Quickshell
import Quickshell.Hyprland
import Quickshell.Io
import "lib/dbus.js" as DBus
import "lib/fcitx.js" as FcitxLib

Singleton {
    id: root

    readonly property string service: "org.fcitx.Fcitx5"

    property bool present: false
    property string group: ""
    property list<string> ims: []
    property string current: ""
    // The group's composing input methods: every one but a keyboard's.
    readonly property var engines: FcitxLib.engines(root.ims)

    // What the chip shows: the engine in use, else the xkb layout code.
    function chip(xkb: string): string {
        return root.present ? FcitxLib.chip(root.current, root.engines, xkb) : xkb;
    }

    // The chip's click: toggle fcitx5 between the keyboard and the engine
    // when the group has one, else switch the xkb layout. Returns which.
    function click(): string {
        const what = root.present ? FcitxLib.click(root.engines) : "xkb-next";
        if (what === "toggle")
            toggleProc.running = true;
        else
            KbLayout.next();
        return what;
    }

    // The keyboard status's fcitx5 part: " im:<input method>" while fcitx5
    // is present, else nothing, so a host without it reads as before.
    function status(): string {
        return root.present ? " im:" + (root.current !== "" ? root.current : "-") : "";
    }

    function refresh(): void {
        if (root.present)
            groupProc.running = true;
    }

    function _call(member: string, args: var): var {
        return ["dbus-send", "--session", "--print-reply", "--dest=" + root.service,
            "/controller", "org.fcitx.Fcitx.Controller1." + member].concat(args);
    }

    function _setPresent(p: bool): void {
        if (root.present === p)
            return;
        root.present = p;
        if (p) {
            root.refresh();
        } else {
            root.group = "";
            root.ims = [];
            root.current = "";
        }
    }

    property var _parse: DBus.initialState()

    Process {
        id: watchProc

        running: true
        command: ["dbus-monitor", "--session", DBus.ownerChangeRule(root.service),
            "type='signal',interface='org.fcitx.Fcitx.Controller1',member='InputMethodGroupsChanged'"]

        stdout: SplitParser {
            onRead: line => {
                if (/ member=InputMethodGroupsChanged$/.test(line))
                    root.refresh();
                const r = DBus.step(root._parse, line);
                root._parse = r.state;
                if (r.event === null)
                    return;
                if (r.event.kind === "subscribed")
                    presenceProc.running = true;
                else if (r.event.name === root.service)
                    root._setPresent(r.event.newOwner !== "");
            }
        }
    }

    // Asked once, as soon as the monitor is live, so an owner taken before
    // the subscription is not missed.
    Process {
        id: presenceProc

        command: ["dbus-send", "--session", "--print-reply", "--dest=org.freedesktop.DBus",
            "/org/freedesktop/DBus", "org.freedesktop.DBus.NameHasOwner", "string:" + root.service]

        stdout: StdioCollector {
            onStreamFinished: root._setPresent(DBus.hasOwnerReply(text))
        }
    }

    Process {
        id: groupProc

        command: root._call("CurrentInputMethodGroup", [])

        stdout: StdioCollector {
            onStreamFinished: {
                root.group = FcitxLib.strings(text.split("\n"))[0] ?? "";
                if (root.group !== "")
                    infoProc.running = true;
            }
        }
    }

    Process {
        id: infoProc

        command: root._call("InputMethodGroupInfo", ["string:" + root.group])

        stdout: StdioCollector {
            onStreamFinished: {
                root.ims = FcitxLib.groupInfo(text.split("\n")).ims;
                currentProc.running = true;
            }
        }
    }

    Process {
        id: currentProc

        command: root._call("CurrentInputMethod", [])

        stdout: StdioCollector {
            onStreamFinished: root.current = FcitxLib.currentIM(text.split("\n"))
        }
    }

    Process {
        id: toggleProc

        command: root._call("Toggle", [])

        onRunningChanged: {
            if (!running)
                currentProc.running = true;
        }
    }

    Connections {
        target: Hyprland

        function onRawEvent(event: HyprlandEvent): void {
            if (event.name === "activewindow" && root.present)
                currentProc.running = true;
        }
    }
}

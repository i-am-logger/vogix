pragma ComponentBehavior: Bound
// Tailnet state from `tailscale status --json`: self + peers, online first.
import QtQuick
import QtQuick.Layouts
import Quickshell.Io
import qs.Panels
import qs.Services
import qs.Vogix

ColumnLayout {
    id: root

    property string self: ""
    property var peers: []

    Component.onCompleted: proc.running = true

    spacing: 10

    PanelLabel {
        text: "Tailscale"
        font.bold: true
        color: Tokens.color("popup", "accent")
    }

    PanelLabel {
        text: root.self === "" ? "not running" : root.self
        color: root.self === ""
            ? Tokens.color("popup", "muted")
            : Tokens.color("popup", "foreground")
    }

    ListView {
        Layout.fillWidth: true
        Layout.fillHeight: true
        Layout.minimumHeight: 180
        clip: true
        spacing: 2
        model: root.peers

        delegate: RowLayout {
            id: row

            required property var modelData

            width: ListView.view.width

            PanelLabel {
                Layout.fillWidth: true
                text: (row.modelData.online ? "󰄴 " : "󰄰 ") + row.modelData.name
            }

            PanelLabel {
                text: row.modelData.ip
                color: Tokens.color("popup", "muted")
            }
        }
    }

    Process {
        id: proc
        command: ["tailscale", "status", "--json"]
        stdout: StdioCollector {
            onStreamFinished: {
                try {
                    const doc = JSON.parse(text);
                    const name = h => (h ?? "").replace(/\.$/, "");
                    root.self = doc.Self
                        ? name(doc.Self.DNSName) + "  " + ((doc.Self.TailscaleIPs ?? [])[0] ?? "")
                        : "";
                    root.peers = Object.values(doc.Peer ?? {}).map(p => ({
                        name: name(p.DNSName).split(".")[0],
                        ip: (p.TailscaleIPs ?? [])[0] ?? "",
                        online: p.Online ?? false,
                    })).sort((a, b) => (b.online - a.online) || a.name.localeCompare(b.name));
                } catch (e) {}
            }
        }
    }
}

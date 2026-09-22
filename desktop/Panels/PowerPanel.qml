pragma ComponentBehavior: Bound
// Battery detail, power profile switching (powerprofilesctl) and a line of
// system info. Each row reports only what its daemon confirmed: no UPower
// means battery state unknown, and the profile row appears once
// power-profiles-daemon has answered, highlighting the profile it reports.
import QtQuick
import QtQuick.Layouts
import Quickshell.Io
import qs.Panels
import qs.Services
import qs.Vogix

ColumnLayout {
    id: root

    readonly property list<string> profiles: ["power-saver", "balanced", "performance"]

    // The daemon's own answer: what `powerprofilesctl get` printed. It
    // prints a profile name only when the daemon answered (a failure goes
    // to stderr, a missing binary prints nothing), so a known name IS the
    // availability signal.
    property string profile: ""
    readonly property bool profilesAvailable: profiles.includes(profile)

    property string uname: ""

    Component.onCompleted: {
        profileProc.running = true;
        unameProc.running = true;
    }

    function setProfile(p: string): void {
        setProc.command = ["powerprofilesctl", "set", p];
        setProc.running = true;
    }

    spacing: 10

    PanelLabel {
        text: "Power"
        font.bold: true
        color: Tokens.color("popup", "accent")
    }

    PanelLabel {
        visible: Battery.available && Battery.present
        text: {
            const pct = Math.round(Battery.percentage * 100);
            const dev = Battery.device;
            const mins = Math.round(((Battery.charging ? dev?.timeToFull : dev?.timeToEmpty) ?? 0) / 60);
            const eta = mins > 0
                ? "  ·  " + Math.floor(mins / 60) + "h" + (mins % 60) + "m "
                    + (Battery.charging ? "to full" : "left")
                : "";
            return "󰁹 " + pct + "%  ·  " + (Battery.charging ? "charging" : "discharging") + eta;
        }
    }

    PanelLabel {
        visible: Battery.available && !Battery.present
        text: "On mains power"
        color: Tokens.color("popup", "muted")
    }

    PanelLabel {
        Layout.fillWidth: true
        visible: !Battery.available
        text: "Battery status unknown: UPower is not running"
        color: Tokens.color("popup", "muted")
        wrapMode: Text.Wrap
    }

    RowLayout {
        Layout.fillWidth: true
        visible: root.profilesAvailable
        spacing: 10

        Repeater {
            model: root.profiles

            PanelLabel {
                id: profRow

                required property string modelData

                text: profRow.modelData
                color: root.profile === profRow.modelData
                    ? Tokens.color("popup", "accent")
                    : Tokens.color("popup", "muted")

                MouseArea {
                    anchors.fill: parent
                    enabled: !setProc.running
                    onClicked: root.setProfile(profRow.modelData)
                }
            }
        }
    }

    PanelLabel {
        Layout.fillWidth: true
        visible: !root.profilesAvailable
        text: "Power profiles unavailable: power-profiles-daemon is not running"
        color: Tokens.color("popup", "muted")
        wrapMode: Text.Wrap
    }

    PanelLabel {
        Layout.fillWidth: true
        text: root.uname
        color: Tokens.color("popup", "muted")
        wrapMode: Text.Wrap
    }

    Process {
        id: profileProc
        command: ["powerprofilesctl", "get"]
        stdout: StdioCollector {
            onStreamFinished: root.profile = text.trim()
        }
    }

    // The highlight follows the daemon, never the click: whatever the set
    // did (a refused profile included), the row re-reads the result.
    Process {
        id: setProc
        onRunningChanged: {
            if (!running)
                profileProc.running = true;
        }
    }

    Process {
        id: unameProc
        command: ["uname", "-snrm"]
        stdout: StdioCollector {
            onStreamFinished: root.uname = text.trim()
        }
    }
}

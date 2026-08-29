pragma ComponentBehavior: Bound
// Battery detail, power profile switching (powerprofilesctl) and a line of
// system info.
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import qs.Panels
import qs.Services
import qs.Vogix

ColumnLayout {
    id: root

    property string profile: ""
    property string uname: ""

    Component.onCompleted: {
        profileProc.running = true;
        unameProc.running = true;
    }

    function setProfile(p: string): void {
        Quickshell.execDetached(["powerprofilesctl", "set", p]);
        root.profile = p;
    }

    spacing: 10

    PanelLabel {
        text: "Power"
        font.bold: true
        color: Tokens.color("popup", "accent")
    }

    PanelLabel {
        visible: Battery.present
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
        visible: !Battery.present
        text: "On mains power"
        color: Tokens.color("popup", "muted")
    }

    RowLayout {
        Layout.fillWidth: true
        spacing: 10

        Repeater {
            model: ["power-saver", "balanced", "performance"]

            PanelLabel {
                id: profRow

                required property string modelData

                text: profRow.modelData
                color: root.profile === profRow.modelData
                    ? Tokens.color("popup", "accent")
                    : Tokens.color("popup", "muted")

                MouseArea {
                    anchors.fill: parent
                    onClicked: root.setProfile(profRow.modelData)
                }
            }
        }
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

    Process {
        id: unameProc
        command: ["uname", "-snrm"]
        stdout: StdioCollector {
            onStreamFinished: root.uname = text.trim()
        }
    }
}

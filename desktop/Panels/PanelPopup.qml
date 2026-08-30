// The one panel popup window: anchored under the bar's end, hosting
// whichever panel qs.Services.Panels has open. Escape closes.
import QtQuick
import Quickshell
import Quickshell.Wayland
import qs.Services
import qs.Vogix

PanelWindow {
    id: root

    visible: Panels.open !== ""
    anchors {
        top: true
        right: true
    }
    // exclusionMode Ignore places this from the raw screen edge, so the
    // live bar thicknesses are added by hand — a parked bar reclaims the
    // space immediately.
    margins.top: BarState.thickness("top") + 8
    margins.right: BarState.thickness("right") + 8
    implicitWidth: 380
    implicitHeight: Math.min(560, content.implicitHeight + 26)
    color: "transparent"
    exclusionMode: ExclusionMode.Ignore
    WlrLayershell.layer: WlrLayer.Overlay
    WlrLayershell.keyboardFocus: visible
        ? WlrKeyboardFocus.OnDemand
        : WlrKeyboardFocus.None

    Rectangle {
        anchors.fill: parent
        radius: 10
        color: Tokens.color("popup", "background")
        border.width: 1
        border.color: Tokens.color("popup", "border")

        Loader {
            id: content
            anchors {
                fill: parent
                margins: 12
            }
            source: {
                switch (Panels.open) {
                case "audio": return "AudioPanel.qml";
                case "network": return "NetworkPanel.qml";
                case "bluetooth": return "BluetoothPanel.qml";
                case "power": return "PowerPanel.qml";
                case "monitor": return "MonitorPanel.qml";
                case "tailscale": return "TailscalePanel.qml";
                case "calendar": return "CalendarPanel.qml";
                case "weather": return "WeatherPanel.qml";
                case "agents": return "AgentsPanel.qml";
                default: return "";
                }
            }
        }

        Keys.onEscapePressed: Panels.close()
    }
}

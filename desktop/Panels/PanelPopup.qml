// The one panel popup window, hosting whichever panel qs.Services.Panels
// has open: beside the bar whose widget summoned it, centred on that
// widget, on that bar's screen; a panel opened by the verb sits under the
// top bar's end on the focused monitor. Escape closes.
import QtQuick
import Quickshell
import Quickshell.Hyprland
import Quickshell.Wayland
import qs.Geometry
import qs.Services
import qs.Vogix

PanelWindow {
    id: root

    // exclusionMode Ignore places this from the raw screen edge, so the
    // live bar thicknesses are insets — a parked bar reclaims its space
    // immediately.
    readonly property rect area: Placement.freeArea(
        Qt.size(root.screen?.width ?? 0, root.screen?.height ?? 0),
        BarState.thickness("top"), BarState.thickness("bottom"),
        BarState.thickness("left"), BarState.thickness("right"))
    readonly property point origin: Placement.popupOrigin(
        Panels.anchorEdge, Panels.anchorRect,
        Qt.size(root.implicitWidth, root.implicitHeight), root.area)

    screen: {
        if (Panels.anchorScreen)
            return Panels.anchorScreen;
        const name = Hyprland.focusedMonitor?.name ?? "";
        return Quickshell.screens.find(s => s.name === name) ?? Quickshell.screens[0] ?? null;
    }

    visible: Panels.open !== ""
    anchors {
        top: true
        left: true
    }
    margins.top: origin.y
    margins.left: origin.x
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
                case "audio-out": return "AudioOutPanel.qml";
                case "audio-in": return "AudioInPanel.qml";
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

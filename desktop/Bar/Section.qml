pragma ComponentBehavior: Bound
// One bar section: the widgets desktop.json names, in order. An unknown
// name renders LOUD (the config is Nix-generated — seeing it is a bug).
import QtQuick
import QtQuick.Layouts

RowLayout {
    id: root

    property list<string> names

    spacing: 12

    Repeater {
        model: root.names

        Loader {
            required property string modelData

            Layout.alignment: Qt.AlignVCenter
            source: {
                switch (modelData) {
                case "workspaces": return "widgets/Workspaces.qml";
                case "window": return "widgets/WindowTitle.qml";
                case "clock": return "widgets/Clock.qml";
                case "mode": return "widgets/ModeIndicator.qml";
                case "theme": return "widgets/ThemeIndicator.qml";
                case "audio": return "widgets/VolumeWidget.qml";
                case "mic": return "widgets/MicWidget.qml";
                case "battery": return "widgets/BatteryWidget.qml";
                case "network": return "widgets/NetworkWidget.qml";
                case "bluetooth": return "widgets/BluetoothWidget.qml";
                case "media": return "widgets/MediaWidget.qml";
                case "tray": return "widgets/TrayWidget.qml";
                case "weather": return "widgets/WeatherWidget.qml";
                case "cpu": return "widgets/CpuWidget.qml";
                case "memory": return "widgets/MemoryWidget.qml";
                case "dnd": return "widgets/DndWidget.qml";
                case "indicators": return "widgets/IndicatorsWidget.qml";
                case "tailscale": return "widgets/TailscaleWidget.qml";
                default: return "widgets/Unknown.qml";
                }
            }
        }
    }
}

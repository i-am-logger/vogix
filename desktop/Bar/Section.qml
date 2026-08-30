pragma ComponentBehavior: Bound
// One bar section: the widgets desktop.json names, in order, laid along
// the bar's axis. An unknown OR misplaced (horizontal-only on a vertical
// bar) name renders LOUD — the config is Nix-generated, so seeing it is
// a generator bug, not user error. The axis context is injected
// post-load into any widget that declares `property BarAxis axis`.
import QtQuick
import QtQuick.Layouts
import qs.Bar.widgets

GridLayout {
    id: root

    property list<string> names
    property BarAxis axis: null

    readonly property bool vertical: axis?.vertical ?? false
    // These read horizontally (window titles, scrolling media text); the
    // Nix side asserts them off vertical bars, and the shell backstops
    // with the loud unknown tile.
    readonly property list<string> horizontalOnly: ["window", "media", "weather", "theme"]

    flow: vertical ? GridLayout.TopToBottom : GridLayout.LeftToRight
    rowSpacing: 12
    columnSpacing: 12

    Repeater {
        model: root.names

        Loader {
            required property string modelData

            Layout.alignment: root.vertical ? Qt.AlignHCenter : Qt.AlignVCenter
            source: {
                if (root.vertical && root.horizontalOnly.includes(modelData))
                    return "widgets/Unknown.qml";
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
                case "update": return "widgets/UpdateWidget.qml";
                default: return "widgets/Unknown.qml";
                }
            }
            onLoaded: {
                // Duck-typed injection: bracket access, because the static
                // item type here is just Item.
                if ("widgetName" in item)
                    item["widgetName"] = modelData;
                if ("axis" in item)
                    item["axis"] = root.axis;
            }
        }
    }
}

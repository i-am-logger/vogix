pragma ComponentBehavior: Bound
// One bar section: the widgets desktop.json names, in order, laid along
// the bar's axis. An unknown OR misplaced (horizontal-only on a vertical
// bar) name renders LOUD — the config is Nix-generated, so seeing it is
// a generator bug, not user error. The axis context is injected
// post-load into any widget that declares `property BarAxis axis`.
import QtQuick
import QtQuick.Layouts
import qs.Bar.widgets
import qs.Vogix

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
    rowSpacing: Metrics.unit * 3
    columnSpacing: Metrics.unit * 3

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
                case "audio-out-picker": return "widgets/AudioOutPicker.qml";
                case "audio-in-picker": return "widgets/AudioInPicker.qml";
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
                case "uptime": return "widgets/UptimeWidget.qml";
                case "update": return "widgets/UpdateWidget.qml";
                case "spectrum-mini": return "widgets/SpectrumWidget.qml";
                case "spectrum-rail": return "widgets/SpectrumWidget.qml";
                case "spectrum-left": return "widgets/SpectrumWidget.qml";
                case "spectrum-right": return "widgets/SpectrumWidget.qml";
                case "oscilloscope": return "widgets/Oscilloscope.qml";
                case "kbd": return "widgets/KbdWidget.qml";
                case "privacy": return "widgets/PrivacyWidget.qml";
                case "stat-cpu": return "widgets/StatCpu.qml";
                case "stat-gpu": return "widgets/StatGpu.qml";
                case "stat-temp": return "widgets/StatTemp.qml";
                case "stat-mem": return "widgets/StatMem.qml";
                case "stat-swap": return "widgets/StatSwap.qml";
                case "stat-disk": return "widgets/StatDisk.qml";
                case "stat-mounts": return "widgets/StatMounts.qml";
                case "stat-net": return "widgets/StatNet.qml";
                case "vu-out": return "widgets/VuOut.qml";
                case "vu-mic": return "widgets/VuMic.qml";
                case "vu-rail": return "widgets/VuRail.qml";
                case "mic-rail": return "widgets/MicRail.qml";
                case "graph-cpu": return "widgets/GraphCpu.qml";
                case "graph-mem": return "widgets/GraphMem.qml";
                case "graph-net": return "widgets/GraphNet.qml";
                case "graph-disk": return "widgets/GraphDisk.qml";
                case "graph-gpu": return "widgets/GraphGpu.qml";
                case "batteries": return "widgets/BatteriesWidget.qml";
                case "menu": return "widgets/MenuButton.qml";
                case "power-glyph": return "widgets/PowerGlyph.qml";
                case "spacer": return "widgets/Spacer.qml";
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

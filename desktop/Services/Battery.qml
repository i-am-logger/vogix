pragma Singleton
// UPower's display device + low-battery notifications: one warning at 10%,
// critical at 5%, re-armed by charging — through the shell's OWN
// notification server (notify-send round-trips the user bus).
import QtQuick
import Quickshell
import Quickshell.Services.UPower

Singleton {
    id: root

    readonly property var device: UPower.displayDevice
    readonly property bool present: device !== null && (device.isLaptopBattery ?? false)
    readonly property real percentage: device ? device.percentage : 0
    readonly property bool charging: device
        ? device.state === UPowerDeviceState.Charging
          || device.state === UPowerDeviceState.FullyCharged
        : false
    readonly property bool onBattery: UPower.onBattery ?? false

    property int warnedAt: 100

    onChargingChanged: {
        if (charging)
            root.warnedAt = 100;
    }

    onPercentageChanged: {
        if (!root.present || root.charging)
            return;
        const pct = Math.round(root.percentage * 100);
        if (pct <= 5 && root.warnedAt > 5) {
            root.warnedAt = 5;
            Quickshell.execDetached(["notify-send", "-u", "critical",
                "Battery critical", pct + "% — plug in now"]);
        } else if (pct <= 10 && root.warnedAt > 10) {
            root.warnedAt = 10;
            Quickshell.execDetached(["notify-send", "-u", "normal",
                "Battery low", pct + "% remaining"]);
        }
    }
}

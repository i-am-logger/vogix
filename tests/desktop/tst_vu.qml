import QtQuick
import QtTest
import "../../desktop/Services/lib/vu.js" as Vu

TestCase {
    name: "Vu"

    // SPA channel positions, as quickshell reports them.
    readonly property int fl: 3
    readonly property int fr: 4
    // A sink at wpctl 0.5 and 0.25: quickshell's cube-rooted volumes.
    readonly property var volumes: [0.5, 0.25]

    function gains(props, monitor) {
        return Vu.outputGains(props, monitor ?? [fl, fr], [fl, fr], volumes);
    }

    function test_a_virtual_sink_without_monitor_volumes_is_undivided() {
        compare(gains({ "node.name": "null-sink" }), [0.5, 0.25]);
        compare(gains({ "monitor.channel-volumes": "false" }), [0.5, 0.25]);
    }

    function test_a_monitor_that_carries_the_volume_keeps_the_division() {
        compare(gains({ "monitor.channel-volumes": "true" }), [1, 1]);
        compare(gains({ "monitor.channel-volumes": "1" }), [1, 1]);
    }

    function test_a_routed_device_sink_is_left_alone() {
        compare(gains({ "device.id": "42", "card.profile.device": "1" }), [1, 1]);
    }

    function test_a_device_sink_quickshell_divides_is_undivided() {
        // Pro-audio, or no route index: quickshell uses the node's volume.
        compare(gains({ "device.id": "42", "card.profile.device": "1", "device.profile.pro": "true" }),
                [0.5, 0.25]);
        compare(gains({ "device.id": "42" }), [0.5, 0.25]);
        compare(gains({ "device.id": "42", "device.profile.pro": "true", "monitor.channel-volumes": "true" }),
                [1, 1]);
    }

    function test_each_monitor_channel_takes_its_own_volume() {
        compare(gains({}, [fr, fl]), [0.25, 0.5]);
        compare(gains({}, [fl]), [0.5]);
    }

    function test_a_channel_quickshell_did_not_divide_is_left_alone() {
        compare(Vu.outputGains({}, [fl, fr], [fl, fr], [0, 0.5]), [1, 0.5]);
        compare(Vu.outputGains({}, [], [fl, fr], volumes), []);
    }
}

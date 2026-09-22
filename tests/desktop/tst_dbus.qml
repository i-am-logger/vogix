import QtQuick
import QtTest
import "../../desktop/Services/lib/dbus.js" as DBus

TestCase {
    name: "DBus"

    // Output of `dbus-monitor --system <ownerChangeRule(NM)>` as dbus 1.16
    // prints it against dbus-broker (the fallback notice goes to stderr).
    readonly property var acquired: [
        "signal time=1790106942.386152 sender=org.freedesktop.DBus -> destination=:1.410 serial=4294967295 path=/org/freedesktop/DBus; interface=org.freedesktop.DBus; member=NameAcquired",
        "   string \":1.410\"",
    ]

    function ownerChanged(oldOwner, newOwner) {
        return [
            "signal time=1790106950.000001 sender=org.freedesktop.DBus -> destination=(null destination) serial=4294967295 path=/org/freedesktop/DBus; interface=org.freedesktop.DBus; member=NameOwnerChanged",
            "   string \"org.freedesktop.NetworkManager\"",
            "   string \"" + oldOwner + "\"",
            "   string \"" + newOwner + "\"",
        ];
    }

    function feed(lines) {
        let state = DBus.initialState();
        const events = [];
        for (const line of lines) {
            const r = DBus.step(state, line);
            state = r.state;
            if (r.event !== null)
                events.push(r.event);
        }
        return events;
    }

    function test_rule_names_the_service() {
        compare(DBus.ownerChangeRule("org.freedesktop.NetworkManager"),
            "type='signal',sender='org.freedesktop.DBus',interface='org.freedesktop.DBus',"
            + "member='NameOwnerChanged',arg0='org.freedesktop.NetworkManager'");
    }

    // The monitor's own NameAcquired marks the subscription live; its
    // string argument is not an owner change.
    function test_name_acquired_means_subscribed() {
        const events = feed(acquired);
        compare(events.length, 1);
        compare(events[0].kind, "subscribed");
    }

    function test_service_appearing() {
        const events = feed(acquired.concat(ownerChanged("", ":1.52")));
        compare(events.length, 2);
        compare(events[1].kind, "owner");
        compare(events[1].name, "org.freedesktop.NetworkManager");
        compare(events[1].oldOwner, "");
        compare(events[1].newOwner, ":1.52");
    }

    function test_service_leaving() {
        const events = feed(ownerChanged(":1.52", ""));
        compare(events.length, 1);
        compare(events[0].newOwner, "");
    }

    // A header that interrupts an argument list abandons it.
    function test_interrupted_signal_yields_nothing() {
        const lines = ownerChanged("", ":1.52").slice(0, 2).concat(acquired);
        const events = feed(lines);
        compare(events.length, 1);
        compare(events[0].kind, "subscribed");
    }

    function test_name_has_owner_reply() {
        verify(DBus.hasOwnerReply("method return time=1790108273.520804 sender=org.freedesktop.DBus -> destination=:1.414 serial=4294967295 reply_serial=2\n   boolean true\n"));
        verify(!DBus.hasOwnerReply("method return time=1 sender=org.freedesktop.DBus -> destination=:1.4 serial=1 reply_serial=2\n   boolean false\n"));
        verify(!DBus.hasOwnerReply(""));
    }
}

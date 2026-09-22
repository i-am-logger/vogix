import QtQuick
import QtTest
import "../../desktop/Services/lib/screencast.js" as Screencast

TestCase {
    name: "Screencast"

    function feed(payloads) {
        let n = 0;
        for (const p of payloads)
            n = Screencast.step(n, p);
        return n;
    }

    // Hyprland 0.55+ formats the kind by name.
    function test_start_and_stop() {
        compare(feed(["1,monitor"]), 1);
        compare(feed(["1,monitor", "0,monitor"]), 0);
    }

    // Two overlapping sessions: the glyph stays until the last one stops.
    function test_overlapping_sessions() {
        compare(feed(["1,monitor", "1,window", "0,monitor"]), 1);
        compare(feed(["1,monitor", "1,window", "0,monitor", "0,window"]), 0);
    }

    // Older Hyprland posts the owner as a number.
    function test_numeric_kind() {
        compare(feed(["1,0", "1,1"]), 2);
    }

    // A session that started before the shell stops: the count floors.
    function test_unseen_session_never_goes_negative() {
        compare(feed(["0,monitor"]), 0);
        compare(feed(["0,monitor", "1,region"]), 1);
    }

    function test_unknown_payload_changes_nothing() {
        compare(Screencast.step(1, ""), 1);
        compare(Screencast.step(1, "2,monitor"), 1);
    }
}

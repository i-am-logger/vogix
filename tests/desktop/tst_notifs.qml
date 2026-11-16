import QtQuick
import QtTest
import "../../desktop/Services/lib/notifs.js" as Notifs

// A card's actions, its click and its icon, from what the sender gave
// (Desktop Notifications: actions are identifier/label pairs; the "default"
// action is the one a click on the notification invokes, and need not be
// shown).
TestCase {
    name: "Notifs"

    function action(id, text) {
        return { identifier: id, text: text };
    }

    function test_buttons_are_every_action_but_the_default_in_order() {
        const acts = [action("later", "Later"), action("default", "Open"), action("mute", "Mute")];
        compare(Notifs.buttons(acts).map(a => a.identifier), ["later", "mute"]);
    }

    function test_no_actions_no_buttons() {
        compare(Notifs.buttons([]).length, 0);
        compare(Notifs.buttons(undefined).length, 0);
    }

    function test_the_default_action_is_found() {
        const acts = [action("later", "Later"), action("default", "Open")];
        compare(Notifs.defaultAction(acts).identifier, "default");
    }

    function test_without_a_default_a_click_invokes_nothing() {
        compare(Notifs.defaultAction([action("later", "Later")]), null);
        compare(Notifs.defaultAction([]), null);
    }

    function test_the_image_hint_comes_first() {
        compare(Notifs.iconSource({ image: "image://qsimage/1", appIcon: "baud", desktopIcon: "org.baud.terminal" }),
            "image://qsimage/1");
    }

    function test_then_the_app_icon() {
        compare(Notifs.iconSource({ image: "", appIcon: "baud", desktopIcon: "org.baud.terminal" }), "baud");
    }

    function test_then_the_desktop_entry_s_icon() {
        compare(Notifs.iconSource({ image: "", appIcon: "", desktopIcon: "org.baud.terminal" }), "org.baud.terminal");
    }

    function test_else_no_icon() {
        compare(Notifs.iconSource({ image: "", appIcon: "", desktopIcon: "" }), "");
    }
}

import QtQuick
import QtTest
import "../../desktop/Services/lib/fcitx.js" as Fcitx

// fcitx5's Controller1 replies as dbus-send --print-reply prints them, and
// what the language chip shows and does with them.
TestCase {
    name: "Fcitx"

    // Captured from fcitx5 5.1.22 on the session bus.
    readonly property var current: [
        "method return time=1794798453.467070 sender=:1.1578 -> destination=:1.6126 serial=63 reply_serial=2",
        "   string \"keyboard-us\"",
    ]
    readonly property var keyboardOnly: [
        "method return time=1794798453.468476 sender=:1.1578 -> destination=:1.6127 serial=64 reply_serial=2",
        "   string \"us\"",
        "   array [",
        "      struct {",
        "         string \"keyboard-us\"",
        "         string \"\"",
        "      }",
        "   ]",
    ]
    // The same reply's format with an engine added to the group (the
    // capture's lines, a second struct naming pinyin).
    readonly property var withEngine: [
        "method return time=1794798453.468476 sender=:1.1578 -> destination=:1.6127 serial=64 reply_serial=2",
        "   string \"us\"",
        "   array [",
        "      struct {",
        "         string \"keyboard-us\"",
        "         string \"\"",
        "      }",
        "      struct {",
        "         string \"pinyin\"",
        "         string \"\"",
        "      }",
        "   ]",
    ]

    function test_the_current_input_method_is_read() {
        compare(Fcitx.currentIM(current), "keyboard-us");
    }

    function test_the_group_s_input_methods_are_read() {
        const g = Fcitx.groupInfo(withEngine);
        compare(g.layout, "us");
        compare(g.ims, ["keyboard-us", "pinyin"]);
    }

    function test_keyboard_input_methods_are_not_engines() {
        compare(Fcitx.engines(Fcitx.groupInfo(keyboardOnly).ims), []);
        compare(Fcitx.engines(Fcitx.groupInfo(withEngine).ims), ["pinyin"]);
    }

    function test_without_fcitx5_the_chip_is_the_layout_and_a_click_switches_it() {
        compare(Fcitx.chip(null, [], "us"), "us");
        compare(Fcitx.click([]), "xkb-next");
    }

    function test_with_only_a_keyboard_input_method_nothing_changes() {
        compare(Fcitx.chip("keyboard-us", [], "il"), "il");
        compare(Fcitx.click([]), "xkb-next");
    }

    function test_an_active_engine_names_the_chip_and_a_click_toggles() {
        compare(Fcitx.chip("pinyin", ["pinyin"], "us"), "pinyin");
        compare(Fcitx.click(["pinyin"]), "toggle");
    }

    function test_an_engine_not_active_leaves_the_layout_and_a_click_still_toggles() {
        compare(Fcitx.chip("keyboard-us", ["pinyin"], "us"), "us");
        compare(Fcitx.click(["pinyin"]), "toggle");
    }
}

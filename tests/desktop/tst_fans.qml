import QtQuick
import QtTest
import "../../desktop/Services/lib/fans.js" as Fans

TestCase {
    name: "Fans"

    // fan-probe.sh lines, as the probe test pins them.
    function fan(path, chip, index, label, max) {
        return [path, chip, index, label, max].join("\t");
    }

    readonly property string board: [
        fan("/sys/class/hwmon/hwmon3/fan1_input", "nct6799", "1", "-", "-"),
        fan("/sys/class/hwmon/hwmon3/fan2_input", "nct6799", "2", "-", "-"),
    ].join("\n")
    readonly property string gpu: fan("/sys/class/hwmon/hwmon1/fan1_input", "amdgpu", "1", "-", "3300")
    readonly property string laptop: fan("/sys/class/hwmon/hwmon5/fan1_input", "dell_smm", "1", "Processor Fan", "4900")

    function test_probe_lines_parse_into_fans() {
        const fans = Fans.parseProbe(board + "\n" + laptop + "\n\nbad line\n");
        compare(fans.length, 3);
        compare(fans[0].key, "/sys/class/hwmon/hwmon3/fan1_input");
        compare(fans[0].chip, "nct6799");
        compare(fans[0].label, "");
        compare(fans[0].maxRpm, 0);
        compare(fans[2].label, "Processor Fan");
        compare(fans[2].maxRpm, 4900);
    }

    function test_rpm_reading() {
        compare(Fans.rpm("1234\n"), 1234);
        compare(Fans.rpm("0"), 0);
        compare(Fans.rpm(""), null);
    }

    function test_titles_prefer_the_chip_label() {
        const fans = Fans.parseProbe(board + "\n" + laptop);
        compare(Fans.title(fans[0], false), "FAN1");
        compare(Fans.title(fans[2], false), "PROCESSOR FAN");
        compare(Fans.title(fans[2], true), "PROC");
    }

    // Headers past nine keep their number on a rail instead of reading as
    // FAN1.
    function test_vertical_titles_keep_two_digit_headers_distinct() {
        const fans = Fans.parseProbe(fan("/x/fan10_input", "nct6799", "10", "-", "-")
            + "\n" + fan("/x/fan1_input", "nct6799", "1", "-", "-"));
        compare(Fans.title(fans[0], true), "F10");
        compare(Fans.title(fans[1], true), "FAN1");
        compare(Fans.title(fans[0], false), "FAN10");
    }

    // A four-character cut never ends on the separator it landed on.
    function test_vertical_titles_trim_a_trailing_separator() {
        const fans = Fans.parseProbe(fan("/x/fan1_input", "asus", "1", "cpu_fan", "-"));
        compare(Fans.title(fans[0], true), "CPU");
    }

    function test_chip_names_show_only_across_chips() {
        verify(!Fans.spansChips(Fans.parseProbe(board)));
        verify(Fans.spansChips(Fans.parseProbe(board + "\n" + gpu)));
    }

    // Empty headers never spin and stay out; a fan that spun once stays
    // in, in probe order, even when it stops.
    function test_only_fans_that_have_spun_are_listed() {
        const fans = Fans.parseProbe(board + "\n" + gpu);
        const seen = {};
        compare(Fans.spinning(fans, seen), []);
        seen["/sys/class/hwmon/hwmon1/fan1_input"] = true;
        seen["/sys/class/hwmon/hwmon3/fan1_input"] = true;
        compare(Fans.spinning(fans, seen),
            ["/sys/class/hwmon/hwmon3/fan1_input", "/sys/class/hwmon/hwmon1/fan1_input"]);
    }

    function test_gauge_needs_the_chip_maximum() {
        const fans = Fans.parseProbe(board + "\n" + gpu);
        compare(Fans.gauge(fans[0], 900), -1);
        fuzzyCompare(Fans.gauge(fans[2], 1650), 0.5, 1e-9);
        compare(Fans.gauge(fans[2], 9000), 1);
    }
}

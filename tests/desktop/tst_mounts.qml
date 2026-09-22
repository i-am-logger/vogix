import QtQuick
import QtTest
import "../../desktop/Services/lib/mounts.js" as Mounts

TestCase {
    name: "Mounts"

    // The impermanence layout: a tmpfs root, the storage on btrfs
    // subvolumes and a vfat /boot.
    readonly property string impermanence: [
        "Use% Type  Mounted on",
        "  1% tmpfs /",
        " 51% btrfs /nix",
        " 51% btrfs /persist",
        " 12% vfat  /boot",
        "",
    ].join("\n")

    function gauges(df, points, hasSwap) {
        const read = Mounts.parseDf(df, points.filter(p => p.startsWith("/")));
        return Mounts.present(points, read.usage, Mounts.inMemory(read.rootFsType), hasSwap);
    }

    function test_rows_parse_as_fractions() {
        const read = Mounts.parseDf(impermanence, ["/", "/nix", "/persist", "/boot"]);
        compare(read.usage["/nix"], 0.51);
        compare(read.usage["/boot"], 0.12);
        compare(read.rootFsType, "tmpfs");
    }

    // A RAM-backed root is scratch: its gauge is omitted, the rest stay
    // in their configured order.
    function test_a_ram_backed_root_has_no_gauge() {
        compare(gauges(impermanence, ["/", "/nix", "/persist", "/boot", "swap"], true),
            ["/nix", "/persist", "/boot", "swap"]);
        compare(gauges(" 3% ramfs /\n", ["/"], false), []);
    }

    // One disk-backed root (a laptop on ext4): the root is the gauge.
    function test_a_disk_backed_root_is_a_gauge() {
        compare(gauges(" 40% ext4 /\n", ["/", "/nix", "swap"], false), ["/"]);
    }

    // df answers a path that is not a mount point with the filesystem
    // containing it; that row names another target and does not count,
    // and a path df does not report at all is simply absent.
    function test_only_the_asked_target_counts() {
        const read = Mounts.parseDf(" 40% ext4 /\n", ["/", "/persist", "/vogix-smoke-absent"]);
        compare(Object.keys(read.usage), ["/"]);
        compare(Mounts.present(["/persist", "/vogix-smoke-absent"], read.usage, false, false), []);
    }

    // A target with spaces, and a header or garbage line, parse as df
    // prints them.
    function test_targets_with_spaces_and_noise() {
        const read = Mounts.parseDf("Use% Type Mounted on\n 7% ext4 /mnt/My Disk\nnot a row\n", ["/mnt/My Disk"]);
        compare(read.usage["/mnt/My Disk"], 0.07);
        compare(read.rootFsType, "");
    }

    function test_swap_follows_its_existence() {
        compare(Mounts.present(["swap"], {}, false, false), []);
        compare(Mounts.present(["swap"], {}, false, true), ["swap"]);
    }
}

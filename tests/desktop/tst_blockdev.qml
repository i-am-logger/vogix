import QtQuick
import QtTest
import "../../desktop/Services/lib/blockdev.js" as BlockDev

TestCase {
    name: "BlockDev"

    // block-devices.sh on a LUKS + LVM host with swap on zram.
    readonly property string probe: [
        "mount\t/\tdm-1",
        "mount\t/nix\tdm-2",
        "mount\t/boot\tnvme0n1p1",
        "mount\t/mnt/My\\x20Disk\tsda1",
        "swap\tzram0",
        "swap\tdm-3",
        "disk\tnvme0n1",
        "disk\tsda",
        "disk\tmmcblk0",
        "",
    ].join("\n")

    function test_mapped_devices_keep_their_kernel_names() {
        const found = BlockDev.parse(probe);
        compare(found.devices["/"], "dm-1");
        compare(found.devices["/nix"], "dm-2");
        compare(found.devices["/boot"], "nvme0n1p1");
    }

    function test_escaped_targets_are_decoded() {
        compare(BlockDev.parse(probe).devices["/mnt/My Disk"], "sda1");
        compare(BlockDev.unescape("/a\\x20b\\x5cc"), "/a b\\c");
    }

    function test_first_swap_device_backs_the_swap_gauge() {
        compare(BlockDev.parse(probe).devices["swap"], "zram0");
    }

    function test_whole_disks_are_listed() {
        compare(BlockDev.parse(probe).disks, ["nvme0n1", "sda", "mmcblk0"]);
    }

    // diskstats: fields 6 and 10 are sectors read and written.
    readonly property string diskstats: [
        " 259       0 nvme0n1 100 0 1000 0 50 0 500 0 0 0 0",
        " 259       1 nvme0n1p1 1 0 10 0 1 0 5 0 0 0 0",
        " 259       2 nvme0n1p2 99 0 990 0 49 0 495 0 0 0 0",
        " 252       1 dm-1 90 0 900 0 40 0 400 0 0 0 0",
        " 179       0 mmcblk0 7 0 70 0 3 0 30 0 0 0 0",
        " 253       0 zram0 4 0 40 0 2 0 20 0 0 0 0",
    ].join("\n")

    // The total counts whole disks once; the encrypted root's own rate
    // comes from its dm device.
    function test_total_counts_whole_disks_only() {
        const r = BlockDev.sectors(diskstats, ["nvme0n1", "mmcblk0"], ["dm-1", "zram0"]);
        compare(r.total, 1000 + 500 + 70 + 30);
        compare(r.perDev["dm-1"], 1300);
        compare(r.perDev["zram0"], 60);
        compare(r.perDev["nvme0n1p2"], undefined);
    }

    function test_no_disks_means_no_total() {
        compare(BlockDev.sectors(diskstats, [], []).total, 0);
    }
}

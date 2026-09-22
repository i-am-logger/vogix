import QtQuick
import QtTest
import "../../desktop/Services/lib/gpu.js" as Gpu

TestCase {
    name: "Gpu"

    // gpu-probe.sh lines, as the probe test pins them.
    function card(name, driver, control, bootVga, pci, busy, idle, smi) {
        return [name, driver, control, bootVga, pci, busy, idle, smi].join("\t");
    }

    readonly property string amdIgpu: card("card1", "amdgpu", "on", "1", "0000:78:00.0",
        "/sys/class/drm/card1/device/gpu_busy_percent", "-", "0")
    readonly property string intelIgpu: card("card0", "i915", "auto", "1", "0000:00:02.0",
        "-", "/sys/class/drm/card0/gt/gt0/rc6_residency_ms", "1")

    function test_window_keeps_the_newest_samples() {
        let w = [];
        for (const v of [0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7])
            w = Gpu.pushWindow(w, v, 5);
        compare(w.length, 5);
        compare(w[0], 0.3);
        compare(w[4], 0.7);
    }

    function test_window_does_not_mutate_its_input() {
        const w = [0.1, 0.2];
        const out = Gpu.pushWindow(w, 0.3, 5);
        compare(w.length, 2);
        compare(out.length, 3);
    }

    function test_mean_of_no_samples_is_zero() {
        compare(Gpu.mean([]), 0);
    }

    // A bimodal idle counter (single digits, then saturated) publishes its
    // level, not whichever extreme the last read landed on.
    function test_bimodal_counter_reads_as_its_level() {
        let w = [];
        for (const v of [0.05, 1.0, 1.0, 0.05, 1.0, 1.0, 1.0, 0.05])
            w = Gpu.pushWindow(w, v, 5);
        fuzzyCompare(Gpu.mean(w), (1.0 + 0.05 + 1.0 + 1.0 + 0.05) / 5, 1e-9);
        verify(Gpu.mean(w) < 0.9);
        verify(Gpu.mean(w) > 0.05);
    }

    function test_probe_lines_parse_into_cards() {
        const cards = Gpu.parseProbe(amdIgpu + "\n" + intelIgpu + "\n\ngarbage\n");
        compare(cards.length, 2);
        compare(cards[0].driver, "amdgpu");
        compare(cards[0].busyPath, "/sys/class/drm/card1/device/gpu_busy_percent");
        compare(cards[0].idlePath, "");
        compare(cards[0].smi, false);
        compare(cards[1].idlePath, "/sys/class/drm/card0/gt/gt0/rc6_residency_ms");
        compare(cards[1].smi, true);
    }

    function test_amd_desktop_reads_the_busy_percent() {
        const pick = Gpu.choose(Gpu.parseProbe(amdIgpu));
        compare(pick.source, Gpu.Source.BusyPercent);
        compare(pick.path, "/sys/class/drm/card1/device/gpu_busy_percent");
    }

    // PRIME sync: the always-on dGPU renders, so it is the one measured.
    function test_prime_sync_measures_the_nvidia_gpu() {
        const nv = card("card1", "nvidia", "on", "0", "0000:01:00.0", "-", "-", "1");
        const pick = Gpu.choose(Gpu.parseProbe(intelIgpu + "\n" + nv));
        compare(pick.source, Gpu.Source.NvidiaSmi);
        compare(pick.pci, "0000:01:00.0");
    }

    // Offload with runtime PM: sampling the dGPU would keep it awake, so
    // the iGPU that drives the display is measured instead.
    function test_runtime_suspendable_dgpu_is_left_alone() {
        const nv = card("card1", "nvidia", "auto", "0", "0000:01:00.0", "-", "-", "1");
        const pick = Gpu.choose(Gpu.parseProbe(intelIgpu + "\n" + nv));
        compare(pick.source, Gpu.Source.IdleResidency);
        compare(pick.path, "/sys/class/drm/card0/gt/gt0/rc6_residency_ms");
    }

    function test_amd_hybrid_skips_the_suspendable_dgpu() {
        const dgpu = card("card0", "amdgpu", "auto", "0", "0000:03:00.0",
            "/sys/class/drm/card0/device/gpu_busy_percent", "-", "0");
        const pick = Gpu.choose(Gpu.parseProbe(dgpu + "\n" + amdIgpu));
        compare(pick.path, "/sys/class/drm/card1/device/gpu_busy_percent");
    }

    function test_nvidia_without_nvidia_smi_falls_through() {
        const nv = card("card1", "nvidia", "on", "0", "0000:01:00.0", "-", "-", "0");
        const intel = card("card0", "i915", "on", "1", "0000:00:02.0",
            "-", "/sys/class/drm/card0/power/rc6_residency_ms", "0");
        compare(Gpu.choose(Gpu.parseProbe(intel + "\n" + nv)).source, Gpu.Source.IdleResidency);
    }

    function test_no_measurable_gpu_is_none() {
        const nouveau = card("card0", "nouveau", "on", "1", "0000:01:00.0", "-", "-", "0");
        compare(Gpu.choose(Gpu.parseProbe(nouveau)).source, Gpu.Source.None);
        compare(Gpu.choose(Gpu.parseProbe("")).source, Gpu.Source.None);
    }

    function test_busy_percent_text() {
        fuzzyCompare(Gpu.busyPercent("42\n"), 0.42, 1e-9);
        compare(Gpu.busyPercent("100"), 1);
        compare(Gpu.busyPercent(""), null);
    }

    // 250 ms idle in a 1000 ms interval is 75% busy.
    function test_residency_is_the_complement_of_idle_share() {
        fuzzyCompare(Gpu.residencyBusy(1000, 1250, 1000), 0.75, 1e-9);
        compare(Gpu.residencyBusy(1000, 2000, 1000), 0);
        // Rounding can put idle slightly past wall time; clamp, never negative.
        compare(Gpu.residencyBusy(1000, 2004, 1000), 0);
    }

    function test_residency_without_an_interval_is_no_sample() {
        compare(Gpu.residencyBusy(-1, 500, 1000), null);
        compare(Gpu.residencyBusy(4294967000, 20, 1000), null);
        compare(Gpu.residencyBusy(100, 200, 0), null);
    }

    function test_nvidia_smi_lines() {
        fuzzyCompare(Gpu.nvidiaSample("37\n"), 0.37, 1e-9);
        compare(Gpu.nvidiaSample(" 0"), 0);
        compare(Gpu.nvidiaSample("[N/A]"), null);
        compare(Gpu.nvidiaSample("[Not Supported]"), null);
        compare(Gpu.nvidiaSample("No devices were found"), null);
    }
}

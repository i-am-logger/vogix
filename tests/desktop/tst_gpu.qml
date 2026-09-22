import QtQuick
import QtTest
import "../../desktop/Services/lib/gpu.js" as Gpu

TestCase {
    name: "Gpu"

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
}

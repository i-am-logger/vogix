// The meter ballistics the VU meters and the spectrum share
// (qs.Components Ballistics): the constants are pinned by value and the
// curve by behaviour, one channel at a time.
import QtQuick
import QtTest
import qs.Components

TestCase {
    name: "Ballistics"

    // One channel's state, advanced `seconds` in ticks of `dt`.
    function run(target: real, state: var, seconds: real, dt: real): var {
        const steps = Math.round(seconds / dt);
        for (let i = 0; i < steps; i++)
            Ballistics.advance([target], state.level, state.cap, state.hold, dt);
        return state;
    }

    function fresh(level: real): var {
        return { level: [level], cap: [level], hold: [0] };
    }

    function test_constants(): void {
        compare(Ballistics.barReleasePerSec, 3.0);
        compare(Ballistics.capHoldSec, 0.53);
        compare(Ballistics.capFallPerSec, 0.75);
        compare(Ballistics.steps, 40);
        // The fleet's 4:1 bar/cap fall ratio.
        compare(Ballistics.barReleasePerSec / Ballistics.capFallPerSec, 4);
    }

    function test_attack_is_instant(): void {
        const s = fresh(0);
        const moved = Ballistics.advance([0.8], s.level, s.cap, s.hold, 0.033);
        verify(moved);
        compare(s.level[0], 0.8);
        compare(s.cap[0], 0.8);
        compare(s.hold[0], Ballistics.capHoldSec);
    }

    function test_bar_releases_full_scale_in_a_third_of_a_second(): void {
        const s = fresh(1);
        run(0, s, 0.1, 0.01);
        fuzzyCompare(s.level[0], 0.7, 1e-9);
        run(0, s, 0.24, 0.01);
        compare(s.level[0], 0);
    }

    function test_cap_holds_then_falls(): void {
        const s = fresh(0);
        Ballistics.advance([1], s.level, s.cap, s.hold, 0.01);
        // Inside the hold the cap stays put while the bar drops away.
        run(0, s, 0.5, 0.01);
        compare(s.cap[0], 1);
        compare(s.level[0], 0);
        // Past the hold it falls at capFallPerSec.
        run(0, s, 0.03 + 0.4, 0.01);
        fuzzyCompare(s.cap[0], 1 - 0.75 * 0.4, 0.02);
        run(0, s, 2, 0.01);
        compare(s.cap[0], 0);
    }

    function test_release_stops_at_the_target(): void {
        const s = fresh(0.9);
        run(0.4, s, 1, 0.033);
        compare(s.level[0], 0.4);
        compare(s.cap[0], 0.4);
    }

    function test_settled_meter_reports_no_movement(): void {
        const s = fresh(0.5);
        s.hold[0] = 0;
        verify(!Ballistics.advance([0.5], s.level, s.cap, s.hold, 0.033));
    }

    function test_channels_advance_independently(): void {
        const level = [0, 1];
        const cap = [0, 1];
        const hold = [0, 0];
        Ballistics.advance([1, 0], level, cap, hold, 0.1);
        compare(level[0], 1);
        fuzzyCompare(level[1], 0.7, 1e-9);
    }

    function test_quantize_to_steps(): void {
        compare(Ballistics.quantize(0.51), 0.5);
        compare(Ballistics.quantize(0.52), 0.525);
        compare(Ballistics.quantize(0), 0);
        compare(Ballistics.quantize(1), 1);
    }
}

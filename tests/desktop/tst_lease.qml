// A widget's Lease on a ref-counted source (qs.Bar.widgets Lease): one
// acquire per rise of `active`, one release per fall or per destruction
// while held, never a second of either.
import QtQuick
import QtTest
import qs.Bar.widgets

TestCase {
    name: "Lease"

    property int refs: 0
    property int acquires: 0
    property int releases: 0

    Component {
        id: leaseComponent

        Lease {
            onAcquire: {
                refs++;
                acquires++;
            }
            onRelease: {
                refs--;
                releases++;
            }
        }
    }

    function init(): void {
        refs = 0;
        acquires = 0;
        releases = 0;
    }

    function test_inactive_lease_holds_nothing(): void {
        const lease = createTemporaryObject(leaseComponent, this, { active: false });
        verify(lease !== null);
        compare(refs, 0);
        lease.destroy();
        wait(0);
        compare(acquires, 0);
        compare(releases, 0);
    }

    function test_created_active_acquires_once(): void {
        const lease = createTemporaryObject(leaseComponent, this, { active: true });
        compare(acquires, 1);
        compare(refs, 1);
        lease.active = true;
        compare(acquires, 1);
    }

    function test_follows_active_edges(): void {
        const lease = createTemporaryObject(leaseComponent, this, { active: false });
        lease.active = true;
        compare(refs, 1);
        lease.active = false;
        compare(refs, 0);
        lease.active = false;
        compare(releases, 1);
        lease.active = true;
        lease.active = false;
        compare(acquires, 2);
        compare(releases, 2);
        compare(refs, 0);
    }

    function test_destruction_releases_a_held_lease(): void {
        const lease = createTemporaryObject(leaseComponent, this, { active: true });
        compare(refs, 1);
        lease.destroy();
        wait(0);
        compare(refs, 0);
        compare(releases, 1);
    }

    function test_destruction_after_release_releases_nothing_more(): void {
        const lease = createTemporaryObject(leaseComponent, this, { active: true });
        lease.active = false;
        lease.destroy();
        wait(0);
        compare(releases, 1);
        compare(refs, 0);
    }
}

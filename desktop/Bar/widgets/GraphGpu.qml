// GPU busy history (SysStat's source: nvidia-smi, amdgpu busy percent or
// Intel idle residency) — absent entirely on hosts with none.
import QtQuick
import qs.Bar.widgets
import qs.Services
import qs.Vogix

GraphCell {
    id: root

    visible: SysStat.hasGpu
    title: "GPU"
    values: SysStat.gpuHistory
    valueText: Math.round(SysStat.gpuBusy * 100) + "%"
    lineColor: Theme.semantic.highlight ?? Tokens.color("bar", "accent")

    // Held while this bar is on screen.
    Lease {
        active: root.axis?.live ?? false
        onAcquire: SysStat.acquire(["gpu"])
        onRelease: SysStat.release(["gpu"])
    }
}

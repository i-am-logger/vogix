// GPU busy history (amdgpu/intel gpu_busy_percent) — absent entirely on
// hosts without the sysfs knob.
import QtQuick
import qs.Bar.widgets
import qs.Services
import qs.Vogix

GraphCell {
    visible: SysStat.hasGpu
    title: "GPU"
    values: SysStat.gpuHistory
    valueText: Math.round(SysStat.gpuBusy * 100) + "%"
    lineColor: Theme.semantic.highlight ?? Tokens.color("bar", "accent")
}

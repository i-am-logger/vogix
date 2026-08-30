// CPU load history panel for the rail.
import QtQuick
import qs.Bar.widgets
import qs.Services

GraphCell {
    title: "CPU"
    values: SysStat.cpuHistory
    valueText: Math.round(SysStat.cpu * 100) + "%"
}

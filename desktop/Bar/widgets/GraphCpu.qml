// CPU load history graph for the rail.
import QtQuick
import qs.Bar.widgets
import qs.Services

GraphCell {
    label: "CPU"
    values: SysStat.cpuHistory
}

// Memory history graph for the rail.
import QtQuick
import qs.Bar.widgets
import qs.Services

GraphCell {
    label: "MEM"
    values: SysStat.memoryHistory
}

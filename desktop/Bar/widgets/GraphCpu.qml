// CPU load history — untitled: it sits beside the CPU stat cell.
import QtQuick
import qs.Bar.widgets
import qs.Services

GraphCell {
    values: SysStat.cpuHistory
}

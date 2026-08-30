// Memory history panel for the rail — link-colored, like the board.
import QtQuick
import qs.Bar.widgets
import qs.Services
import qs.Vogix

GraphCell {
    title: "MEM"
    values: SysStat.memoryHistory
    valueText: Math.round(SysStat.memory * 100) + "%"
    lineColor: Theme.semantic.link ?? Tokens.color("bar", "accent")
}

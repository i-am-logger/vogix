// Memory history — untitled beside the MEM stat cell; link-colored.
import QtQuick
import qs.Bar.widgets
import qs.Services
import qs.Vogix

GraphCell {
    values: SysStat.memoryHistory
    lineColor: Theme.semantic.link ?? Tokens.color("bar", "accent")
}

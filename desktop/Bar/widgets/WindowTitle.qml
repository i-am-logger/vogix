// The focused window's title.
import QtQuick
import Quickshell.Hyprland
import qs.Bar.widgets

BarText {
    text: Hyprland.activeToplevel?.title ?? ""
    elide: Text.ElideRight
    maximumLineCount: 1
}

// The focused window's title — quiet chrome, not a headline.
import QtQuick
import Quickshell.Hyprland
import qs.Bar.widgets
import qs.Vogix

BarText {
    text: Hyprland.activeToplevel?.title ?? ""
    elide: Text.ElideRight
    maximumLineCount: 1
    color: Tokens.color("bar", "muted")
}

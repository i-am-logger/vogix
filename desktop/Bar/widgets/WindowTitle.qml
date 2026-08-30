// The focused window's title — quiet chrome, not a headline. Read from
// the ActiveWindow service (Hyprland.activeToplevel is null on this
// compositor — no toplevel-mapping protocol).
import QtQuick
import qs.Bar.widgets
import qs.Services
import qs.Vogix

BarText {
    text: ActiveWindow.title
    elide: Text.ElideRight
    maximumLineCount: 1
    color: Tokens.color("bar", "muted")
}
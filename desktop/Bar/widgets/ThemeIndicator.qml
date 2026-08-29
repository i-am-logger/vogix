// Current theme + variant, straight from theme.json.
import QtQuick
import qs.Bar.widgets
import qs.Vogix

BarText {
    text: Theme.name + (Theme.variant !== "" ? "/" + Theme.variant : "")
    color: Tokens.color("bar", "muted")
}

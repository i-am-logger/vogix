// Shared text style for bar widgets — the shell font from desktop.json.
// `axis` arrives from Section on a bar; widgets branch on axis?.vertical
// for their stacked forms. Null everywhere else (popups, previews).
import QtQuick
import qs.Vogix

Text {
    property BarAxis axis: null

    color: Tokens.color("bar", "foreground")
    font.family: Config.fontFamily
    font.pixelSize: Metrics.body
    verticalAlignment: Text.AlignVCenter
}

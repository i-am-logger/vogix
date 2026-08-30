// Shared text style for bar widgets — the shell font from desktop.json.
import QtQuick
import qs.Vogix

Text {
    color: Tokens.color("bar", "foreground")
    font.family: Config.fontFamily
    font.pixelSize: Metrics.body
    verticalAlignment: Text.AlignVCenter
}

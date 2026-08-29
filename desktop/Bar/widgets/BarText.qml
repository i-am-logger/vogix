// Shared text style for bar widgets — the shell font from desktop.json.
import QtQuick
import qs.Vogix

Text {
    color: Tokens.color("bar", "foreground")
    font.family: Config.fontFamily
    font.pixelSize: Config.fontSize
    verticalAlignment: Text.AlignVCenter
}

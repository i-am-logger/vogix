// Shared text style for panel rows.
import QtQuick
import qs.Vogix

Text {
    color: Tokens.color("popup", "foreground")
    font.family: Config.fontFamily
    font.pixelSize: Metrics.body
    elide: Text.ElideRight
}

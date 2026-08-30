// A terse uppercase HUD label — CPU, MEM, OUT, LANG. Letter-spaced micro
// type; color comes from the caller (a Tokens.color of its surface).
import QtQuick
import qs.Vogix

Text {
    font.family: Config.fontFamily
    font.pixelSize: Metrics.caption
    font.letterSpacing: 1.5
    font.capitalization: Font.AllUppercase
    verticalAlignment: Text.AlignVCenter
}

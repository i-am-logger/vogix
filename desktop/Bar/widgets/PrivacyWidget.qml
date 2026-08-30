// Capture indicators: appear ONLY while something is recording — a mic
// glyph for live audio capture, a screen glyph when a cast is detected.
import QtQuick
import qs.Bar.widgets
import qs.Services
import qs.Vogix

BarText {
    visible: Privacy.micInUse || Privacy.screencast
    text: (Privacy.micInUse ? "󰍬" : "") + (Privacy.screencast ? " 󰻃" : "")
    color: Tokens.color("bar", "urgent")
}

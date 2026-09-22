// Capture indicators: appear ONLY while something is recording — a mic
// glyph for live audio capture, a screen glyph while a screencast runs.
import QtQuick
import qs.Bar.widgets
import qs.Services
import qs.Vogix

BarText {
    visible: Privacy.micInUse || Privacy.screencast
    text: [Privacy.micInUse ? "󰍬" : "", Privacy.screencast ? "󰻃" : ""]
        .filter(g => g !== "").join(" ")
    color: Tokens.color("bar", "urgent")
}

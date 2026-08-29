// The weather forecast popup (wttrbar's tooltip, tags stripped).
import QtQuick
import QtQuick.Layouts
import qs.Panels
import qs.Services
import qs.Vogix

ColumnLayout {
    spacing: 8

    PanelLabel {
        text: "Weather"
        font.bold: true
        color: Tokens.color("popup", "accent")
    }

    PanelLabel {
        Layout.fillWidth: true
        text: Weather.forecast !== "" ? Weather.forecast : "no forecast yet"
        color: Weather.forecast !== ""
            ? Tokens.color("popup", "foreground")
            : Tokens.color("popup", "muted")
        wrapMode: Text.Wrap
    }
}

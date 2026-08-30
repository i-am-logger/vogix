// A widget name desktop.json carries but this section cannot render —
// unknown, or horizontal-only on a vertical bar. Loud, because the
// config is Nix-generated and seeing this is a generator bug.
import QtQuick
import qs.Bar.widgets

BarText {
    property string widgetName: "widget"

    text: "?" + widgetName
    color: "#ff00ff"
}

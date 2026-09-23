// A widget name desktop.json carries but its section cannot render: not
// in the registry, on a bar of the orientation the registry keeps it
// off, or an undefined custom cell. Loud, because the config is
// Nix-generated and seeing this is a generator bug; Section logs the
// reason beside it.
import QtQuick
import qs.Bar.widgets

BarText {
    property string widgetName: "widget"

    text: "?" + widgetName
    color: "#ff00ff"
}

// Current weather from wttrbar; click opens the forecast panel.
import QtQuick
import qs.Bar.widgets
import qs.Services

BarText {
    id: root

    visible: Weather.enabled && Weather.text !== ""
    text: Weather.text

    MouseArea {
        anchors.fill: parent
        onClicked: root.axis?.togglePanel("weather", root) ?? Panels.toggle("weather")
    }
}

// Current weather from wttrbar; click opens the forecast panel.
import QtQuick
import qs.Bar.widgets
import qs.Services

BarText {
    visible: Weather.enabled && Weather.text !== ""
    text: Weather.text

    MouseArea {
        anchors.fill: parent
        onClicked: Panels.toggle("weather")
    }
}

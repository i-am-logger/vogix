// The vertical rail's VU pair: OUT and MIC side by side, meters running
// bottom-up like a mixing console.
import QtQuick
import qs.Bar.widgets

Row {
    spacing: 6

    VuOut {
        vertical: true
    }

    VuMic {
        vertical: true
    }
}

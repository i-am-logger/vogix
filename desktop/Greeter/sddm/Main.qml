// The vogix SDDM greeter: one centered auth card over the theme background,
// keyboard-first, colored ENTIRELY by semantic slots. Colors arrive the SDDM
// way — theme.conf [General] keys rendered at build from the first vogix
// user's palette (config.<key>) — and, when /var/lib/vogix/greeter carries a
// newer runtime-synced theme.json (`vogix greeter sync`), that wins.
//
// Plain QtQuick on the SDDM context properties (config, userModel,
// sessionModel, sddm) — no Quickshell imports here: SDDM hosts its own Qt6
// engine inside the greeter compositor.
import QtQuick
import QtQuick.Controls.Basic

Rectangle {
    id: root

    // Palette: theme.conf values, overridden by the runtime-synced theme.json
    // when present and readable. Missing key = loud magenta, like Tokens.qml.
    property var runtime: null

    function slot(name: string): color {
        if (runtime && runtime[name])
            return runtime[name];
        const v = config[name];
        return (v && v !== "") ? v : "#ff00ff";
    }

    property string wallpaper: (runtime && runtime.__wallpaper) ? runtime.__wallpaper : (config.wallpaper ?? "")
    property bool authFailed: false

    width: 1920
    height: 1080
    color: slot("background")

    Component.onCompleted: {
        // Runtime follow (opt-in on the host): the greeter group syncs
        // theme.json + background.json here on every theme switch. XHR over
        // file:// needs QML_XHR_ALLOW_FILE_READ=1 (GreeterEnvironment);
        // any failure leaves the build-time palette in place.
        try {
            const req = new XMLHttpRequest();
            req.onreadystatechange = function () {
                if (req.readyState !== XMLHttpRequest.DONE)
                    return;
                try {
                    const doc = JSON.parse(req.responseText);
                    if (doc.semantic)
                        root.runtime = doc.semantic;
                } catch (e) {}
            };
            req.open("GET", "file:///var/lib/vogix/greeter/theme.json");
            req.send();

            const bg = new XMLHttpRequest();
            bg.onreadystatechange = function () {
                if (bg.readyState !== XMLHttpRequest.DONE)
                    return;
                try {
                    const doc = JSON.parse(bg.responseText);
                    if (doc.path && root.runtime)
                        root.runtime.__wallpaper = doc.path;
                } catch (e) {}
            };
            bg.open("GET", "file:///var/lib/vogix/greeter/background.json");
            bg.send();
        } catch (e) {}

        // First boot has no remembered user: start in the username field.
        if (username.text === "")
            username.forceActiveFocus();
        else
            password.forceActiveFocus();
    }

    Image {
        anchors.fill: parent
        visible: root.wallpaper !== ""
        source: root.wallpaper
        fillMode: Image.PreserveAspectCrop
    }

    Connections {
        target: sddm

        function onLoginFailed() {
            root.authFailed = true;
            password.text = "";
            message.text = "Login failed";
            shake.start();
            password.forceActiveFocus();
        }

        function onInformationMessage(msg) {
            message.text = msg;
        }
    }

    Rectangle {
        id: card

        anchors.centerIn: parent
        width: 360
        height: col.implicitHeight + 48
        radius: 10
        color: Qt.alpha(slot("background_surface"), 0.98)
        border.width: 1
        border.color: root.authFailed ? slot("danger") : slot("foreground_border")

        SequentialAnimation {
            id: shake
            NumberAnimation { target: card; property: "anchors.horizontalCenterOffset"; to: -12; duration: 50 }
            NumberAnimation { target: card; property: "anchors.horizontalCenterOffset"; to: 12; duration: 90 }
            NumberAnimation { target: card; property: "anchors.horizontalCenterOffset"; to: 0; duration: 50 }
        }

        Column {
            id: col

            anchors {
                left: parent.left
                right: parent.right
                verticalCenter: parent.verticalCenter
                margins: 28
            }
            spacing: 14

            Text {
                width: parent.width
                horizontalAlignment: Text.AlignHCenter
                text: sddm.hostName ?? ""
                color: slot("foreground_heading")
                font.family: config.font ?? "monospace"
                font.pixelSize: 20
                font.bold: true
            }

            TextField {
                id: username
                width: parent.width
                text: userModel.lastUser ?? ""
                placeholderText: "User"
                color: slot("foreground_text")
                placeholderTextColor: slot("foreground_comment")
                font.family: config.font ?? "monospace"
                font.pixelSize: 14
                background: Rectangle {
                    radius: 6
                    color: slot("background_selection")
                }
                onAccepted: password.forceActiveFocus()
            }

            TextField {
                id: password
                width: parent.width
                echoMode: TextInput.Password
                placeholderText: "Password"
                color: slot("foreground_text")
                placeholderTextColor: slot("foreground_comment")
                font.family: config.font ?? "monospace"
                font.pixelSize: 14
                background: Rectangle {
                    radius: 6
                    color: slot("background_selection")
                }
                onTextEdited: root.authFailed = false
                onAccepted: sddm.login(username.text, password.text, sessionRow.index)
            }

            Text {
                id: message
                width: parent.width
                horizontalAlignment: Text.AlignHCenter
                visible: text !== ""
                text: ""
                color: root.authFailed ? slot("danger") : slot("foreground_comment")
                font.family: config.font ?? "monospace"
                font.pixelSize: 12
                wrapMode: Text.Wrap
            }

            // Session picker: hidden with one session; F3 cycles.
            Text {
                id: sessionRow

                property int index: sessionModel.lastIndex

                width: parent.width
                horizontalAlignment: Text.AlignHCenter
                visible: sessionModel.rowCount() > 1
                text: {
                    const idx = sessionModel.index(sessionRow.index, 0);
                    return "session: " + (sessionModel.data(idx, Qt.UserRole + 4) ?? "") + "  (F3)";
                }
                color: slot("foreground_comment")
                font.family: config.font ?? "monospace"
                font.pixelSize: 11
            }
        }
    }

    Text {
        anchors {
            bottom: parent.bottom
            horizontalCenter: parent.horizontalCenter
            bottomMargin: 24
        }
        text: "Enter to log in"
        color: slot("foreground_comment")
        font.family: config.font ?? "monospace"
        font.pixelSize: 11
    }

    Shortcut {
        sequence: "F3"
        onActivated: sessionRow.index = (sessionRow.index + 1) % sessionModel.rowCount()
    }
}

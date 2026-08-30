pragma Singleton
// The audio spectrum: a cava subprocess in raw-ascii mode, ref-counted so
// it runs ONLY while a spectrum widget is on screen, killed (and the bars
// zero-filled) the moment the last one goes. Config is inlined over
// stdin — no file to manage. Frame writes skip when nothing changed, so
// silence costs no repaints.
import QtQuick
import Quickshell
import Quickshell.Io
import qs.Vogix

Singleton {
    id: root

    property int refs: 0

    function acquire(): void {
        refs++;
    }

    function release(): void {
        refs = Math.max(0, refs - 1);
    }

    readonly property var spectrumConf: (Config.doc.meters ?? {}).spectrum ?? ({})
    readonly property int bars: spectrumConf.bars ?? 16
    readonly property bool active: (spectrumConf.enable ?? true) && refs > 0

    // 0..1 per bar, length == bars; zero-filled while inactive.
    property list<real> values: []
    property string _last: ""

    onActiveChanged: {
        if (!active) {
            values = Array(bars).fill(0);
            _last = "";
        }
    }

    Process {
        id: proc

        running: root.active
        command: ["sh", "-c",
            "printf '[general]\\nframerate=25\\nbars=%d\\n[output]\\nmethod=raw\\ndata_format=ascii\\nascii_max_range=100\\n[smoothing]\\nnoise_reduction=35\\nmonstercat=1.5\\n' "
            + root.bars + " | cava -p /dev/stdin"]

        stdout: SplitParser {
            onRead: line => {
                if (line === root._last)
                    return;
                root._last = line;
                root.values = line.split(";").filter(s => s !== "")
                    .map(s => Math.max(0, Math.min(1, Number(s) / 100)));
            }
        }

        // running flips false when the subprocess dies on its own too — an
        // EOF'd cava while a spectrum widget still wants it is worth a line.
        onRunningChanged: {
            if (!running && root.active)
                console.warn("vogix: cava exited while a spectrum widget is visible");
        }
    }
}

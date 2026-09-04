pragma Singleton
// The default sink/source pair, the device rosters behind them, and the
// mic's mute. Watches the default sink to flash the volume OSD — the
// initial binding fire is swallowed, startup must not flash.
import QtQuick
import Quickshell
import Quickshell.Services.Pipewire

Singleton {
    id: root

    readonly property var sink: Pipewire.defaultAudioSink
    readonly property var source: Pipewire.defaultAudioSource
    property bool primed: false

    // The devices pipewire will accept as a default. `type` and the name
    // trio are CONSTANT on an UNBOUND node, so a roster costs no binding
    // — only the default pair below is tracked, because volume and mute
    // are the properties that need it. Streams are excluded because
    // pipewire refuses one as a default (PwDefaultTracker rejects
    // anything without the AudioSink/AudioSource flags), and an
    // AudioOutStream carries the Sink bit.
    readonly property var sinks: [...Pipewire.nodes.values].filter(n =>
        !n.isStream && (n.type & PwNodeType.AudioSink) === PwNodeType.AudioSink)
    readonly property var sources: [...Pipewire.nodes.values].filter(n =>
        !n.isStream && (n.type & PwNodeType.AudioSource) === PwNodeType.AudioSource)

    // Pipewire hands out three names per node and any of them may be
    // empty. `label` is the terse one for the rail, `fullLabel` the
    // verbose one for a surface with room — the rail elides, the panel
    // spells it out.
    function label(node: var): string {
        if (!node)
            return "";
        return node.nickname || node.description || node.name || "";
    }

    function fullLabel(node: var): string {
        if (!node)
            return "";
        return node.description || node.nickname || node.name || "";
    }

    // A PREFERENCE, not a command: pipewire picks the real default and
    // may decline (a device that vanishes mid-switch), so callers read
    // `sink`/`source` back rather than echoing the request.
    function setSink(node: var): void {
        Pipewire.preferredDefaultAudioSink = node;
    }

    function setSource(node: var): void {
        Pipewire.preferredDefaultAudioSource = node;
    }

    // Mic mute, both directions. Deliberately NOT a binding on the
    // node's own `muted`: the first assignment would destroy it and the
    // property would then stop following the hardware. The node's change
    // signal writes it back instead, and the equality guard in each
    // direction is what keeps the pair from ringing.
    property bool micMuted: true

    // No source means no mute to flip: the property stays `true` rather
    // than reporting a live mic no device backs.
    function toggleMic(): void {
        if (root.source?.audio)
            root.micMuted = !root.micMuted;
    }

    function _readMic(): void {
        root.micMuted = root.source?.audio?.muted ?? true;
    }

    onMicMutedChanged: {
        const audio = root.source?.audio ?? null;
        if (audio && audio.muted !== root.micMuted)
            audio.muted = root.micMuted;
    }

    // A new default source carries its own mute; adopt it rather than
    // pushing the old node's state onto it.
    onSourceChanged: root._readMic()
    Component.onCompleted: root._readMic()

    // Volume and mute are invalid on an unbound node, so both defaults
    // are bound — the rosters above are not, and must not be.
    PwObjectTracker {
        objects: [root.sink, root.source].filter(n => !!n)
    }

    Connections {
        target: root.source?.audio ?? null

        function onMutedChanged() {
            root._readMic();
        }
    }

    Connections {
        target: root.sink?.audio ?? null

        function onVolumesChanged() {
            if (!root.primed) {
                root.primed = true;
                return;
            }
            Osd.show("volume", root.sink.audio.volume, root.sink.audio.muted, "");
        }

        function onMutedChanged() {
            if (root.primed)
                Osd.show("volume", root.sink.audio.volume, root.sink.audio.muted, "");
        }
    }
}

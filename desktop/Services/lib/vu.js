.pragma library
// The output VU's reference. quickshell's peak monitor reports, per channel
// of a sink's monitor, the cube root of the peak divided by the sink's
// cube-rooted channel volume, unless it takes the volume from the device's
// route (peak.cpp; PwNode::shouldUseDevice). The division assumes the monitor
// carries the volume, which a sink's monitor does only with
// monitor.channel-volumes. Without it, PipeWire's default, the monitor
// carries the signal before the volume, and the divided peak over-reads by
// the volume's attenuation. Pure; Peaks.qml applies the factors.

// A PipeWire boolean property, read as spa_atob reads it.
function flag(value) {
    return value === "true" || value === "1";
}

// Whether quickshell takes the node's volume from its device's route, and so
// leaves the peak undivided: a device node, not in the pro-audio profile,
// with a card.profile.device. quickshell also requires the device to list a
// route for that card.profile.device; routes are not visible here, so such a
// node counts as routed.
function routedVolume(props) {
    return props["device.id"] !== undefined
        && !flag(props["device.profile.pro"])
        && props["card.profile.device"] !== undefined;
}

// Per monitor channel, the factor that turns quickshell's peak into the level
// applications send: the volume quickshell divided by, where the monitor
// never carried it, and 1 everywhere else. quickshell divides each monitor
// channel by the volume of the node channel at the same position, and leaves
// a channel at zero volume undivided.
function outputGains(props, monitorChannels, nodeChannels, nodeVolumes) {
    const undo = !routedVolume(props) && !flag(props["monitor.channel-volumes"]);
    return monitorChannels.map(channel => {
        if (!undo)
            return 1;
        const i = nodeChannels.indexOf(channel);
        const volume = i >= 0 ? nodeVolumes[i] : 0;
        return volume > 0 ? volume : 1;
    });
}

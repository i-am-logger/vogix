// Surface color tokens: { slot, alpha } resolved against the CURRENT
// theme's semantic table — one lookup, identical in QML and the v2 Rust
// shell. A missing token or unknown slot resolves LOUD magenta rather than
// silently inheriting: the config is Nix-generated, so seeing it means a
// generator bug, not user error.
pragma Singleton
import QtQuick
import Quickshell

Singleton {
    function color(surface: string, name: string): color {
        const tokens = (Config.doc.surfaces ?? {})[surface] ?? {};
        const token = tokens[name];
        if (!token)
            return "#ff00ff";
        const hex = Theme.semantic[token.slot];
        if (!hex)
            return "#ff00ff";
        return Qt.alpha(hex, token.alpha ?? 1.0);
    }
}

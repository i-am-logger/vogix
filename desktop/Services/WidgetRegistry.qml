pragma Singleton
// The bar widget registry, Bar/widgets/registry.json: every name a
// desktop.json layout may place, the component beside it that renders the
// name, and whether it reads only horizontally. The Nix layout options
// take their name type from the same file and `vogix desktop check`
// compiles it in, so the shell, the options and the check cannot disagree
// about a name. (A quickshell singleton lives in qs.Services, not beside
// the widgets: qs.Bar.widgets stays loadable without quickshell for
// qmltestrunner.)
import QtQuick
import Quickshell
import Quickshell.Io

Singleton {
    id: root

    // name → { component, horizontalOnly? }
    readonly property var widgets: JSON.parse(file.text()).widgets

    FileView {
        id: file

        path: Qt.resolvedUrl("../Bar/widgets/registry.json")
        // No bar can lay out without it, and it is a small file inside the
        // shell's own tree: read it before the first lookup returns.
        blockLoading: true
    }
}

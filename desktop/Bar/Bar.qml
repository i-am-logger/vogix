pragma ComponentBehavior: Bound
// The HUD: up to four bars on EVERY monitor. Horizontal bars are created
// first — layer-shell arranges later surfaces inside the area earlier
// exclusive zones left over, which is what tucks the vertical rails
// between top and bottom instead of overlapping the corners — so the
// verticals sit behind a loader gated on the horizontals' backing
// windows, with a timer backstop so a wedged horizontal can never keep
// the rails from existing at all.
import QtQuick
import Quickshell
import qs.Vogix

Scope {
    Variants {
        model: Quickshell.screens

        Scope {
            id: screenScope

            required property var modelData

            readonly property bool horizontalsReady:
                (!topBar.visible || topBar.backingWindowVisible)
                && (!bottomBar.visible || bottomBar.backingWindowVisible)

            BarPanel {
                id: topBar
                screen: screenScope.modelData
                edge: "top"
            }

            BarPanel {
                id: bottomBar
                screen: screenScope.modelData
                edge: "bottom"
            }

            Timer {
                id: zoneFallback
                property bool fired: false
                interval: 1000
                running: !screenScope.horizontalsReady && !fired
                onTriggered: fired = true
            }

            LazyLoader {
                active: screenScope.horizontalsReady || zoneFallback.fired

                component: Scope {
                    BarPanel {
                        screen: screenScope.modelData
                        edge: "left"
                    }

                    BarPanel {
                        screen: screenScope.modelData
                        edge: "right"
                    }
                }
            }
        }
    }
}

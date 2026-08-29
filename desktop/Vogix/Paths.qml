// The ONLY file that knows where vogix keeps things — exactly
// Config::state_dir() on the Rust side. No new environment variables.
pragma Singleton
import Quickshell

Singleton {
    readonly property string home: Quickshell.env("HOME") ?? ""
    readonly property string stateRoot:
        (Quickshell.env("XDG_STATE_HOME") ?? (home + "/.local/state")) + "/vogix"
    readonly property string configRoot:
        Quickshell.env("XDG_CONFIG_HOME") ?? (home + "/.config")
}

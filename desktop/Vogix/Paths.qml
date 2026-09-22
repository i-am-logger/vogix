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
    // Session-scoped state (gone at logout and reboot), the directory the
    // Rust side's shader generator uses too.
    readonly property string runtimeRoot:
        (Quickshell.env("XDG_RUNTIME_DIR") ?? "/tmp") + "/vogix"
}

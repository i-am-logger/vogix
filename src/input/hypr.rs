//! Hyprland IPC dispatch over the control socket (`.socket.sock`).
//!
//! The schema's actions are Hyprland config-bind strings (`"movefocus, l"`,
//! `"exec, $TERMINAL"`, `"killactive,"`). To run one we translate it to the
//! socket command form (`dispatch movefocus l`) and write it to Hyprland's
//! request socket — the same thing `hyprctl dispatch` does, but without spawning
//! a process per keystroke.

use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::time::Duration;

/// Translate a schema action (`"dispatcher, args"`) into a socket command
/// (`"dispatch dispatcher args"`).
///
/// The action's first comma separates the dispatcher from its arguments;
/// Hyprland's socket protocol wants them space-separated after `dispatch`.
pub fn action_to_command(action: &str) -> String {
    let a = action.trim();
    match a.split_once(',') {
        Some((disp, args)) => {
            let args = args.trim();
            if args.is_empty() {
                format!("dispatch {}", disp.trim())
            } else {
                format!("dispatch {} {}", disp.trim(), args)
            }
        }
        None => format!("dispatch {a}"),
    }
}

/// A handle to Hyprland's control socket.
#[derive(Debug, Clone)]
pub struct Hypr {
    socket: PathBuf,
}

impl Hypr {
    /// Locate Hyprland's control socket.
    ///
    /// Preferred path: `$XDG_RUNTIME_DIR/hypr/$HIS/.socket.sock` (then the
    /// legacy `/tmp/hypr/...`), where `$HIS` is `HYPRLAND_INSTANCE_SIGNATURE`.
    ///
    /// Fallback when `$HIS` is absent — which is the normal case for a systemd
    /// *user* service, since it does not inherit the compositor's session
    /// environment: scan the per-instance socket directories under
    /// `$XDG_RUNTIME_DIR/hypr` (and `/tmp/hypr`) and pick the most recently
    /// modified live `.socket.sock`. `XDG_RUNTIME_DIR` is always set by systemd
    /// for user units, so this works without any env propagation into the
    /// unit. The daemon discovers its environment rather than depending on it
    /// being injected.
    pub fn discover() -> Option<Self> {
        if let Ok(his) = std::env::var("HYPRLAND_INSTANCE_SIGNATURE") {
            for base in Self::socket_bases() {
                let p = base.join(&his).join(".socket.sock");
                if p.exists() {
                    return Some(Self { socket: p });
                }
            }
        }
        Self::scan_latest_socket()
    }

    /// Directories that hold per-instance Hyprland socket folders, preferred
    /// first.
    fn socket_bases() -> Vec<PathBuf> {
        let mut bases = Vec::new();
        if let Ok(x) = std::env::var("XDG_RUNTIME_DIR") {
            bases.push(PathBuf::from(x).join("hypr"));
        }
        bases.push(PathBuf::from("/tmp/hypr"));
        bases
    }

    /// Scan all instance directories and return the most recently modified
    /// live control socket — the best guess at the active Hyprland when the
    /// instance signature isn't available to name it directly.
    fn scan_latest_socket() -> Option<Self> {
        let mut best: Option<(std::time::SystemTime, PathBuf)> = None;
        for base in Self::socket_bases() {
            let Ok(entries) = std::fs::read_dir(&base) else {
                continue;
            };
            for entry in entries.flatten() {
                let sock = entry.path().join(".socket.sock");
                if !sock.exists() {
                    continue;
                }
                let mtime = entry
                    .metadata()
                    .and_then(|m| m.modified())
                    .unwrap_or(std::time::UNIX_EPOCH);
                let is_newer = match &best {
                    Some((t, _)) => mtime > *t,
                    None => true,
                };
                if is_newer {
                    best = Some((mtime, sock));
                }
            }
        }
        best.map(|(_, socket)| Self { socket })
    }

    /// Use an explicit socket path (for tests / non-standard setups).
    pub fn with_socket(socket: PathBuf) -> Self {
        Self { socket }
    }

    /// The resolved socket path.
    pub fn socket_path(&self) -> &PathBuf {
        &self.socket
    }

    /// Run a schema action by dispatching it over the socket.
    pub fn dispatch(&self, action: &str) -> std::io::Result<()> {
        self.send(&action_to_command(action))
    }

    /// Write a raw command to the control socket and check the reply.
    fn send(&self, command: &str) -> std::io::Result<()> {
        let mut stream = UnixStream::connect(&self.socket)?;
        stream.set_read_timeout(Some(Duration::from_millis(200)))?;
        stream.set_write_timeout(Some(Duration::from_millis(200)))?;
        stream.write_all(command.as_bytes())?;
        // Hyprland replies "ok" on success, or an error string. Treat a
        // non-empty, non-"ok" reply as a failure: a stale socket left by a
        // *restarted* compositor frequently still `connect()`s and accepts the
        // write but rejects the dispatch — and without inspecting the reply that
        // silent drop looks like success. That is the "keybindings stopped
        // working after Hyprland restarted" symptom: the engine keeps dispatching
        // into a dead instance. Returning Err here lets the caller drop the stale
        // handle and re-discover the live socket. An empty / timed-out read is
        // tolerated as ok so a merely slow reply doesn't churn re-discovery.
        let mut buf = Vec::new();
        let _ = stream.read_to_end(&mut buf);
        let reply = String::from_utf8_lossy(&buf);
        let reply = reply.trim();
        if !reply.is_empty() && reply != "ok" {
            return Err(std::io::Error::other(format!(
                "hyprland rejected '{command}': {reply}"
            )));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn focus_action_becomes_dispatch() {
        assert_eq!(action_to_command("movefocus, l"), "dispatch movefocus l");
    }

    #[test]
    fn no_arg_action_drops_trailing_comma() {
        assert_eq!(action_to_command("killactive,"), "dispatch killactive");
        assert_eq!(action_to_command("fullscreen"), "dispatch fullscreen");
    }

    #[test]
    fn exec_and_workspace_and_resize() {
        assert_eq!(
            action_to_command("exec, $TERMINAL"),
            "dispatch exec $TERMINAL"
        );
        assert_eq!(action_to_command("workspace, 1"), "dispatch workspace 1");
        assert_eq!(
            action_to_command("resizeactive, -40 0"),
            "dispatch resizeactive -40 0"
        );
    }

    #[test]
    fn whitespace_is_normalized() {
        assert_eq!(
            action_to_command("  movewindow ,  l "),
            "dispatch movewindow l"
        );
    }
}

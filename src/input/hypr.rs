//! Hyprland IPC dispatch over the control socket (`.socket.sock`).
//!
//! The schema's actions are Hyprland config-bind strings (`"movefocus, l"`,
//! `"exec, $TERMINAL"`, `"killactive,"`). To run one we translate it to the
//! socket command form (`dispatch movefocus l`) and write it to Hyprland's
//! request socket — the same thing `hyprctl dispatch` does, but without spawning
//! a process per keystroke.

use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
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
    /// Locate a **live** Hyprland control socket.
    ///
    /// Candidates, in priority order: the instance named by
    /// `$HYPRLAND_INSTANCE_SIGNATURE` (the compositor that launched us), then
    /// every other instance socket newest-modified first. Each candidate is
    /// connection-tested ([`is_live`]) and dead ones are skipped — `$HIS` is
    /// NOT trusted blindly. After a Hyprland crash/restart the dead instance's
    /// `.socket.sock` lingers on disk and the stale signature still in our
    /// environment would otherwise pin the engine to that dead compositor
    /// forever (the "keybindings stop after Hyprland restarts" bug); falling
    /// through to the newest *live* socket re-attaches to the restarted
    /// compositor with no service restart. `$HIS` is also commonly absent for a
    /// systemd *user* unit, where the newest-live scan is the only path —
    /// `XDG_RUNTIME_DIR` is always set, so this needs no env propagation.
    pub fn discover() -> Option<Self> {
        Self::candidate_sockets()
            .into_iter()
            .find(|sock| Self::is_live(sock))
            .map(|socket| Self { socket })
    }

    /// Candidate control-socket paths in priority order (see [`Self::discover`]).
    fn candidate_sockets() -> Vec<PathBuf> {
        let mut out: Vec<PathBuf> = Vec::new();
        if let Ok(his) = std::env::var("HYPRLAND_INSTANCE_SIGNATURE") {
            for base in Self::socket_bases() {
                out.push(base.join(&his).join(".socket.sock"));
            }
        }
        // Every instance socket, newest-modified first — covers a restarted
        // compositor whose new signature isn't in our environment yet.
        let mut dated: Vec<(std::time::SystemTime, PathBuf)> = Vec::new();
        for base in Self::socket_bases() {
            let Ok(entries) = std::fs::read_dir(&base) else {
                continue;
            };
            for entry in entries.flatten() {
                let sock = entry.path().join(".socket.sock");
                if !sock.exists() || out.contains(&sock) {
                    continue;
                }
                let mtime = entry
                    .metadata()
                    .and_then(|m| m.modified())
                    .unwrap_or(std::time::UNIX_EPOCH);
                dated.push((mtime, sock));
            }
        }
        dated.sort_by(|a, b| b.0.cmp(&a.0));
        out.extend(dated.into_iter().map(|(_, sock)| sock));
        out
    }

    /// True when a Hyprland is actually listening on `socket`. A lingering
    /// socket file left by a crashed instance still `exists()` but refuses the
    /// connection (`ECONNREFUSED`), so this is what distinguishes a live
    /// compositor from a dead one's leftover node.
    fn is_live(socket: &Path) -> bool {
        UnixStream::connect(socket).is_ok()
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

    // Regression: a Hyprland crash/restart leaves the dead instance's
    // `.socket.sock` file on disk. `discover()` must treat that lingering node
    // as NOT live, otherwise a stale $HYPRLAND_INSTANCE_SIGNATURE pins the
    // engine to the dead compositor and keybindings stay dead until the service
    // is restarted by hand.
    #[test]
    fn is_live_rejects_lingering_dead_socket() {
        use std::os::unix::net::UnixListener;
        let sock =
            std::env::temp_dir().join(format!(".vogix-hypr-islive-{}.sock", std::process::id()));
        let _ = std::fs::remove_file(&sock);

        // No socket node at all → not live.
        assert!(!Hypr::is_live(&sock));

        // A listening (live) compositor → live.
        let listener = UnixListener::bind(&sock).expect("bind test socket");
        assert!(Hypr::is_live(&sock));

        // Listener gone but the file lingers (the crashed-instance case) → the
        // connection is refused, so it must read as not live.
        drop(listener);
        assert!(!Hypr::is_live(&sock));

        let _ = std::fs::remove_file(&sock);
    }
}

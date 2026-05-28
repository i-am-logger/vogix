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
    /// Locate Hyprland's control socket from the environment, if running.
    ///
    /// Tries `$XDG_RUNTIME_DIR/hypr/$HIS/.socket.sock` (current) then
    /// `/tmp/hypr/$HIS/.socket.sock` (legacy), where `$HIS` is
    /// `HYPRLAND_INSTANCE_SIGNATURE`.
    pub fn discover() -> Option<Self> {
        let his = std::env::var("HYPRLAND_INSTANCE_SIGNATURE").ok()?;
        let candidates = [
            std::env::var("XDG_RUNTIME_DIR").ok().map(|x| {
                PathBuf::from(x)
                    .join("hypr")
                    .join(&his)
                    .join(".socket.sock")
            }),
            Some(PathBuf::from("/tmp/hypr").join(&his).join(".socket.sock")),
        ];
        candidates
            .into_iter()
            .flatten()
            .find(|p| p.exists())
            .map(|socket| Self { socket })
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

    /// Write a raw command to the control socket and drain the reply.
    fn send(&self, command: &str) -> std::io::Result<()> {
        let mut stream = UnixStream::connect(&self.socket)?;
        stream.set_read_timeout(Some(Duration::from_millis(200)))?;
        stream.set_write_timeout(Some(Duration::from_millis(200)))?;
        stream.write_all(command.as_bytes())?;
        // Hyprland replies "ok" (or an error message); read and ignore it so the
        // connection closes cleanly.
        let mut buf = Vec::new();
        let _ = stream.read_to_end(&mut buf);
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

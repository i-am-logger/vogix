//! Vogix daemon — auto-restore session on start, auto-save on window changes,
//! plus submap-mode telemetry to help diagnose modal-keybinding ergonomics.
//!
//! Connects to Hyprland's IPC socket to watch for window events.
//! On start: restores the last saved session.
//! On window open/close/move: auto-saves the current session.
//! On submap change: appends a line to ~/.local/state/vogix/modes.log with
//!   the previous mode's dwell time. Short dwells often indicate canceled
//!   or accidental mode entries — useful for spotting muscle-memory mismatches.
//! On SIGTERM (shutdown/reboot): final save before exit.

use crate::commands::session::{handle_session_restore, handle_session_save, session_path};
use crate::errors::Result;
use crate::state::State;
use log::{error, info, warn};
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

/// Find Hyprland's event socket path
fn hyprland_event_socket() -> Option<String> {
    let his = std::env::var("HYPRLAND_INSTANCE_SIGNATURE").ok()?;
    let xdg = std::env::var("XDG_RUNTIME_DIR").ok()?;
    Some(format!("{}/hypr/{}/.socket2.sock", xdg, his))
}

/// Does Hyprland currently have any client windows open?
///
/// Tri-state: `Some(true)`/`Some(false)` only when `hyprctl` actually answered;
/// `None` when the compositor was unreachable (hyprctl missing, non-zero exit,
/// or unparseable output — typically because `HYPRLAND_INSTANCE_SIGNATURE` is
/// absent from the daemon's environment).
///
/// The caller MUST NOT treat `None` as "empty desktop". The previous `bool`
/// form collapsed "no windows" and "couldn't ask" into the same `false`, so a
/// daemon that started before the compositor exported its env would conclude
/// the desktop was empty and auto-restore *over* a live session — the
/// destructive clobber this guard exists to prevent.
fn hyprland_has_clients() -> Option<bool> {
    let output = match std::process::Command::new("hyprctl")
        .args(["clients", "-j"])
        .output()
    {
        Ok(o) if o.status.success() => o,
        Ok(o) => {
            warn!(
                "hyprctl clients failed (exit {}): {}",
                o.status,
                String::from_utf8_lossy(&o.stderr).trim()
            );
            return None;
        }
        Err(e) => {
            warn!("hyprctl unavailable ({e}); cannot determine desktop state");
            return None;
        }
    };
    match serde_json::from_slice::<Vec<serde_json::Value>>(&output.stdout) {
        Ok(v) => Some(!v.is_empty()),
        Err(e) => {
            warn!("hyprctl clients: unparseable output ({e})");
            None
        }
    }
}

pub fn handle_daemon() -> Result<()> {
    info!("vogix daemon starting");
    // Surface the daemon's view of the session env up front. If the Wayland /
    // Hyprland vars are absent here, the daemon started before the compositor
    // exported them (the systemd-user ordering race) — which is the upstream
    // cause of "restored apps die" and "modes.log is frozen". Logging it makes
    // that diagnosable from `journalctl --user -u vogix-daemon` without a repro.
    info!(
        "daemon env: WAYLAND_DISPLAY={:?} HYPRLAND_INSTANCE_SIGNATURE={:?} DISPLAY={:?} SSH_AUTH_SOCK={:?} XDG_RUNTIME_DIR={:?}",
        std::env::var("WAYLAND_DISPLAY").ok(),
        std::env::var("HYPRLAND_INSTANCE_SIGNATURE").ok(),
        std::env::var("DISPLAY").ok(),
        std::env::var("SSH_AUTH_SOCK").ok(),
        std::env::var("XDG_RUNTIME_DIR").ok(),
    );

    // Set up SIGTERM handler for clean shutdown (reboot/poweroff)
    let shutdown = Arc::new(AtomicBool::new(false));
    let shutdown_clone = shutdown.clone();
    ctrlc::set_handler(move || {
        info!("Received shutdown signal, saving session...");
        shutdown_clone.store(true, Ordering::SeqCst);
    })
    .ok();

    // Restore session on startup — but ONLY into an empty Hyprland.
    //
    // The daemon restarts on every home-manager activation (i.e. every
    // `rebuild-system`), not just at login/reboot. Restoring on every restart
    // re-spawns terminals + brave on top of the user's live layout, which is
    // destructive (windows pile on workspace 1, terminals duplicate, etc).
    //
    // Heuristic: if hyprctl already reports any clients, the user is mid-work
    // and a restore would clobber. Restore only when the desktop is truly
    // empty — which is the post-reboot/post-login case auto-restore was meant
    // to serve in the first place.
    match hyprland_has_clients() {
        Some(true) => {
            info!("Hyprland has open clients; skipping auto-restore (user is working).");
        }
        None => {
            warn!(
                "Cannot confirm desktop state (compositor unreachable — is \
                 HYPRLAND_INSTANCE_SIGNATURE set in the daemon's env?); skipping \
                 auto-restore to avoid clobbering a live session. Run \
                 `vogix session restore` manually if this was a fresh login."
            );
        }
        Some(false) => {
            let restore_name = if session_path("last").map(|p| p.exists()).unwrap_or(false) {
                "last"
            } else {
                "autosave"
            };
            info!(
                "Empty desktop detected — restoring session '{}'...",
                restore_name
            );
            match handle_session_restore(restore_name, false) {
                Ok(()) => info!("Session '{}' restored", restore_name),
                Err(e) => warn!("No session to restore: {}", e),
            }
        }
    }

    // Connect to Hyprland event socket for auto-save
    let socket_path = match hyprland_event_socket() {
        Some(path) => path,
        None => {
            warn!(
                "HYPRLAND_INSTANCE_SIGNATURE not set — running WITHOUT event monitoring: \
                 auto-save on window changes AND submap modes.log telemetry are DISABLED \
                 for this daemon's lifetime (`vogix modes recent` will go stale). Usually \
                 means the daemon started before the compositor exported its env."
            );
            loop {
                if shutdown.load(Ordering::SeqCst) {
                    save_on_shutdown();
                    return Ok(());
                }
                std::thread::sleep(Duration::from_secs(1));
            }
        }
    };

    info!("Connecting to Hyprland event socket: {}", socket_path);

    // Retry connection (Hyprland might not be ready yet at boot)
    let stream = loop {
        if shutdown.load(Ordering::SeqCst) {
            save_on_shutdown();
            return Ok(());
        }
        match UnixStream::connect(&socket_path) {
            Ok(s) => break s,
            Err(e) => {
                warn!("Waiting for Hyprland socket: {}", e);
                std::thread::sleep(Duration::from_secs(2));
            }
        }
    };

    // Set read timeout so we can check the shutdown flag periodically
    stream.set_read_timeout(Some(Duration::from_secs(5))).ok();

    info!("Connected to Hyprland, watching for window events");

    let reader = BufReader::new(stream);
    let mut last_save = Instant::now();
    let debounce = Duration::from_secs(2); // Debounce rapid events
    let mut pending_save = false;
    let mut mode_tracker = ModeTracker::new();

    for line in reader.lines() {
        if shutdown.load(Ordering::SeqCst) {
            break;
        }

        let line = match line {
            Ok(l) => l,
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                // Timeout — flush pending save if debounce period passed
                if pending_save && last_save.elapsed() >= debounce {
                    if let Err(e) = handle_session_save("autosave") {
                        error!("Auto-save failed: {}", e);
                    }
                    pending_save = false;
                    last_save = Instant::now();
                }
                continue;
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => {
                if pending_save && last_save.elapsed() >= debounce {
                    if let Err(e) = handle_session_save("autosave") {
                        error!("Auto-save failed: {}", e);
                    }
                    pending_save = false;
                    last_save = Instant::now();
                }
                continue;
            }
            Err(e) => {
                error!("Socket read error: {}", e);
                break;
            }
        };

        let mut parts = line.splitn(2, ">>");
        let event = parts.next().unwrap_or("");
        let payload = parts.next().unwrap_or("");

        match event {
            // State-changing events — mark for save
            "openwindow"
            | "closewindow"
            | "movewindow"
            | "changefloatingmode"
            | "movetoworkspace"
            | "movetoworkspacesilent" => {
                info!("Session changed ({}), will save after debounce", event);
                pending_save = true;
                // Save immediately if enough time passed
                if last_save.elapsed() >= debounce {
                    if let Err(e) = handle_session_save("autosave") {
                        error!("Auto-save failed: {}", e);
                    }
                    pending_save = false;
                    last_save = Instant::now();
                }
            }
            // Submap (mode) telemetry — record entries/exits for ergonomics analysis
            "submap" => {
                mode_tracker.record(payload);
            }
            // Compositor shutting down — save immediately
            "configreloaded" | "monitorremoved" => {
                info!("Compositor event ({}), saving immediately", event);
                handle_session_save("autosave").ok();
                last_save = Instant::now();
            }
            _ => {}
        }
    }

    // Final save on exit (shutdown/reboot/socket close)
    save_on_shutdown();
    Ok(())
}

fn save_on_shutdown() {
    info!("Saving session before shutdown...");
    match handle_session_save("autosave") {
        Ok(()) => info!("Session saved to 'autosave'"),
        Err(e) => error!("Failed to save session on shutdown: {}", e),
    }
}

/// Empty submap payload from Hyprland means "back to the default (reset) submap".
/// We normalise to "app" because that's the user-facing root mode in vogix,
/// matching state.toml's `current_mode = "app"`.
fn normalise_submap(payload: &str) -> String {
    let trimmed = payload.trim();
    if trimmed.is_empty() || trimmed == "reset" {
        "app".to_string()
    } else {
        trimmed.to_string()
    }
}

/// The vogix16 SEMANTIC slot used for each mode's border colour.
///
/// Keys are the UNDERSCORE form produced by the vogix16 theme loader
/// (theme/loader/vogix16.rs), e.g. `foreground_border` — not the hyphenated
/// praxis/Nix key. Modes use neutral/accent slots only — never `warning`/
/// `danger`, which are reserved for real warning/error states.
///   app     → foreground_border (neutral resting state)
///   desktop → active  (cyan — "actively in WM mode")
///   move    → link    (blue)
///   resize  → highlight (purple)
///   console → foreground_comment (muted passthrough)
fn mode_border_slot(mode: &str) -> &'static str {
    match mode {
        "desktop" => "active",
        "move" => "link",
        "resize" => "highlight",
        "console" => "foreground_comment",
        _ => "foreground_border",
    }
}

/// "#aabbcc" / "aabbcc" → "rgb(aabbcc)" for Hyprland's col.* keywords.
fn hex_to_hypr_rgb(hex: &str) -> String {
    format!("rgb({})", hex.trim_start_matches('#'))
}

/// Set the Hyprland window border colour to match the current mode, using the
/// CURRENT theme's semantic colours. Runs OFF the keypress path (the daemon
/// reacts to submap-change events) so it can never delay a keybind — that
/// async-on-the-critical-path mistake is exactly what broke momentary mode.
/// Best-effort: any failure (no theme_sources, hyprctl missing) is silently skipped.
fn apply_mode_border(mode: &str) {
    let config = match crate::config::Config::load() {
        Ok(c) => c,
        Err(_) => return,
    };
    let state = match State::load() {
        Ok(s) => s,
        Err(_) => return,
    };
    let colors = match crate::commands::shader::load_current_theme_colors(&config, &state) {
        Ok(c) => c,
        Err(_) => return,
    };
    let active = match colors.get(mode_border_slot(mode)) {
        Some(h) => hex_to_hypr_rgb(h),
        None => return,
    };
    let inactive = colors
        .get("background_selection")
        .map(|h| hex_to_hypr_rgb(h))
        .unwrap_or_else(|| "rgb(313244)".to_string());
    let batch = format!(
        "keyword general:col.active_border {active} ; keyword general:col.inactive_border {inactive}"
    );
    let _ = std::process::Command::new("hyprctl")
        .args(["--batch", &batch])
        .output();
}

/// Tracks submap transitions and writes them to ~/.local/state/vogix/modes.log.
///
/// Format (one transition per line):
///   `2026-05-09T22:01:14.123Z  app -> desktop  (app: 4523ms)`
///
/// Short dwell times in the parenthesised duration are the most useful signal
/// for spotting accidental mode entries — taps that end up immediately exiting
/// (e.g. CapsLock fired while typing) typically show up as <300ms entries.
struct ModeTracker {
    current: String,
    entered_at: Instant,
    log_path: Option<PathBuf>,
    log_resolved: bool,
}

impl ModeTracker {
    fn new() -> Self {
        Self {
            current: "app".to_string(),
            entered_at: Instant::now(),
            log_path: None,
            log_resolved: false,
        }
    }

    fn log_path(&mut self) -> Option<&PathBuf> {
        if !self.log_resolved {
            self.log_resolved = true;
            match State::state_dir() {
                Ok(dir) => {
                    if let Err(e) = std::fs::create_dir_all(&dir) {
                        warn!("modes.log: cannot create state dir: {}", e);
                    } else {
                        self.log_path = Some(dir.join("modes.log"));
                    }
                }
                Err(e) => warn!("modes.log: cannot resolve state dir: {}", e),
            }
        }
        self.log_path.as_ref()
    }

    fn record(&mut self, raw_payload: &str) {
        let next = normalise_submap(raw_payload);
        if next == self.current {
            return;
        }

        // Visual mode indicator: recolour the window borders to match the mode
        // (theme-derived, off the keypress path). See apply_mode_border.
        apply_mode_border(&next);

        let dwell_ms = self.entered_at.elapsed().as_millis();
        let prev = std::mem::replace(&mut self.current, next.clone());
        self.entered_at = Instant::now();

        let timestamp = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        let line = format!(
            "{ts}  {prev} -> {next}  ({prev}: {dwell_ms}ms)\n",
            ts = timestamp,
            prev = prev,
            next = next,
            dwell_ms = dwell_ms,
        );

        info!("Mode: {} -> {} ({}: {}ms)", prev, next, prev, dwell_ms);

        // Sync state.toml so `current_mode` reflects reality. Without this, the
        // CapsLock-tap → Scroll_Lock → submap path bypasses `vogix mode <name>`
        // and leaves state.toml stuck at whatever the last CLI invocation set.
        // Done before the log write because state.toml is the source of truth
        // other code reads — getting it right matters more than the trace line.
        if let Err(e) = State::save_current_mode(&next) {
            warn!("modes: state.toml current_mode sync failed: {}", e);
        }

        let path = match self.log_path() {
            Some(p) => p.clone(),
            None => return,
        };

        match OpenOptions::new().create(true).append(true).open(&path) {
            Ok(mut f) => {
                if let Err(e) = f.write_all(line.as_bytes()) {
                    warn!("modes.log: write failed: {}", e);
                }
            }
            Err(e) => warn!("modes.log: open failed ({}): {}", path.display(), e),
        }
    }
}

#[cfg(test)]
mod tests {
    // Note: env var tests omitted — set_var/remove_var are unsafe in Rust 2024+
    // and cause race conditions in parallel test execution.
    // The socket path logic is tested indirectly via integration tests.

    // ── Property: relevant event detection ──

    #[test]
    fn test_relevant_events_detected() {
        let relevant = [
            "openwindow>>abc,1,class,title",
            "closewindow>>abc",
            "movewindow>>abc,1",
            "changefloatingmode>>abc,1",
        ];
        for event in &relevant {
            assert!(
                event.starts_with("openwindow>>")
                    || event.starts_with("closewindow>>")
                    || event.starts_with("movewindow>>")
                    || event.starts_with("changefloatingmode>>"),
                "Should detect: {}",
                event
            );
        }
    }

    #[test]
    fn test_irrelevant_events_ignored() {
        let irrelevant = [
            "workspace>>1",
            "activewindow>>class,title",
            "fullscreen>>1",
            "monitoradded>>HDMI-1",
            "focusedmon>>HDMI-1",
        ];
        for event in &irrelevant {
            assert!(
                !event.starts_with("openwindow>>")
                    && !event.starts_with("closewindow>>")
                    && !event.starts_with("movewindow>>")
                    && !event.starts_with("changefloatingmode>>"),
                "Should ignore: {}",
                event
            );
        }
    }

    use super::{hex_to_hypr_rgb, mode_border_slot, normalise_submap};

    #[test]
    fn test_normalise_submap_empty_is_app() {
        assert_eq!(normalise_submap(""), "app");
    }

    #[test]
    fn test_normalise_submap_reset_is_app() {
        assert_eq!(normalise_submap("reset"), "app");
    }

    #[test]
    fn test_normalise_submap_named_passes_through() {
        assert_eq!(normalise_submap("desktop"), "desktop");
        assert_eq!(normalise_submap("arrange"), "arrange");
        assert_eq!(normalise_submap("theme"), "theme");
    }

    #[test]
    fn test_normalise_submap_trims_whitespace() {
        assert_eq!(normalise_submap("  desktop\n"), "desktop");
        assert_eq!(normalise_submap("   "), "app");
    }

    #[test]
    fn test_mode_border_slot_uses_semantic_accents_not_status() {
        // Modes map to neutral/accent slots — NEVER warning/danger (reserved).
        // Keys are the underscore form the vogix16 loader emits.
        assert_eq!(mode_border_slot("app"), "foreground_border");
        assert_eq!(mode_border_slot("desktop"), "active");
        assert_eq!(mode_border_slot("move"), "link");
        assert_eq!(mode_border_slot("resize"), "highlight");
        assert_eq!(mode_border_slot("console"), "foreground_comment");
        // Unknown modes fall back to the neutral resting colour.
        assert_eq!(mode_border_slot("whatever"), "foreground_border");
        // Guard the invariant: no mode is allowed to use a status slot.
        for m in ["app", "desktop", "move", "resize", "console", "x"] {
            let slot = mode_border_slot(m);
            assert!(
                !["warning", "danger", "notice", "success"].contains(&slot),
                "mode {m} must not use status slot {slot}"
            );
        }
    }

    #[test]
    fn test_hex_to_hypr_rgb() {
        assert_eq!(hex_to_hypr_rgb("#89b4fa"), "rgb(89b4fa)");
        assert_eq!(hex_to_hypr_rgb("89b4fa"), "rgb(89b4fa)");
    }
}

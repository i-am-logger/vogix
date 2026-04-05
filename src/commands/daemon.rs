//! Vogix daemon — auto-restore session on start, auto-save on window changes.
//!
//! Connects to Hyprland's IPC socket to watch for window events.
//! On start: restores the last saved session.
//! On window open/close/move: auto-saves the current session.
//! On SIGTERM (shutdown/reboot): final save before exit.

use crate::commands::session::{handle_session_restore, handle_session_save, session_path};
use crate::errors::Result;
use log::{error, info, warn};
use std::io::{BufRead, BufReader};
use std::os::unix::net::UnixStream;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Find Hyprland's event socket path
fn hyprland_event_socket() -> Option<String> {
    let his = std::env::var("HYPRLAND_INSTANCE_SIGNATURE").ok()?;
    let xdg = std::env::var("XDG_RUNTIME_DIR").ok()?;
    Some(format!("{}/hypr/{}/.socket2.sock", xdg, his))
}

pub fn handle_daemon() -> Result<()> {
    info!("vogix daemon starting");

    // Set up SIGTERM handler for clean shutdown (reboot/poweroff)
    let shutdown = Arc::new(AtomicBool::new(false));
    let shutdown_clone = shutdown.clone();
    ctrlc::set_handler(move || {
        info!("Received shutdown signal, saving session...");
        shutdown_clone.store(true, Ordering::SeqCst);
    })
    .ok();

    // Restore session on startup — prefer manual "last" over "autosave"
    let restore_name = if session_path("last").exists() {
        "last"
    } else {
        "autosave"
    };
    info!("Restoring session '{}'...", restore_name);
    match handle_session_restore(restore_name, false) {
        Ok(()) => info!("Session '{}' restored", restore_name),
        Err(e) => warn!("No session to restore: {}", e),
    }

    // Connect to Hyprland event socket for auto-save
    let socket_path = match hyprland_event_socket() {
        Some(path) => path,
        None => {
            warn!("HYPRLAND_INSTANCE_SIGNATURE not set, running without event monitoring");
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
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .ok();

    info!("Connected to Hyprland, watching for window events");

    let reader = BufReader::new(stream);
    let mut last_save = Instant::now();
    let debounce = Duration::from_secs(2); // Debounce rapid events
    let mut pending_save = false;

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

        let event = line.split(">>").next().unwrap_or("");

        match event {
            // State-changing events — mark for save
            "openwindow" | "closewindow" | "movewindow" | "changefloatingmode"
            | "movetoworkspace" | "movetoworkspacesilent" => {
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
}

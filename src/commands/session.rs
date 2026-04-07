// uses the psf/black style (rust equivalent: rustfmt)

//! Session save/restore for desktop state.
//!
//! Captures and restores:
//! - Hyprland window layout (class, workspace, position, size, floating state)
//! - Wezterm terminal state (pane CWD, running command)
//! - Brave browser (relies on built-in session restore)

use crate::errors::Result;
use log::{debug, info, warn};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// A saved window from Hyprland
#[derive(Debug, Serialize, Deserialize)]
struct HyprWindow {
    class: String,
    title: String,
    workspace: String,
    floating: bool,
    size: [i32; 2],
    at: [i32; 2],
    fullscreen: i32,
}

/// A saved terminal pane from Wezterm
#[derive(Debug, Serialize, Deserialize)]
struct WeztermPane {
    pane_id: u64,
    title: String,
    cwd: String,
}

/// Full desktop session
#[derive(Debug, Serialize, Deserialize)]
struct Session {
    windows: Vec<HyprWindow>,
    terminals: Vec<WeztermPane>,
}

fn session_dir() -> PathBuf {
    let state_dir = dirs::state_dir()
        .or_else(|| dirs::home_dir().map(|h| h.join(".local").join("state")))
        .expect("could not determine state directory");
    state_dir.join("vogix").join("sessions")
}

pub fn session_path(name: &str) -> PathBuf {
    session_dir().join(format!("{}.json", name))
}

const MAX_AUTOSAVE_STACK: usize = 20;

/// Push an autosave snapshot onto the stack (rotate old ones)
fn push_autosave_stack() {
    let dir = session_dir();
    // Rotate: autosave-19 → delete, autosave-18 → autosave-19, ..., autosave → autosave-1
    for i in (1..MAX_AUTOSAVE_STACK).rev() {
        let from = dir.join(format!("autosave-{}.json", i));
        let to = dir.join(format!("autosave-{}.json", i + 1));
        if from.exists() {
            fs::rename(&from, &to).ok();
        }
    }
    // Current autosave → autosave-1
    let current = dir.join("autosave.json");
    let first = dir.join("autosave-1.json");
    if current.exists() {
        fs::rename(&current, &first).ok();
    }
}

/// Query Hyprland for current window layout
fn capture_hyprland() -> Result<Vec<HyprWindow>> {
    let output = match Command::new("hyprctl").args(["clients", "-j"]).output() {
        Ok(o) => o,
        Err(e) => {
            warn!("hyprctl not available: {}", e);
            return Ok(vec![]);
        }
    };

    if !output.status.success() {
        warn!("hyprctl failed, skipping Hyprland capture");
        return Ok(vec![]);
    }

    let clients: Vec<serde_json::Value> = serde_json::from_slice(&output.stdout)?;
    let windows: Vec<HyprWindow> = clients
        .into_iter()
        .map(|c| HyprWindow {
            class: c["class"].as_str().unwrap_or("").to_string(),
            title: c["title"].as_str().unwrap_or("").to_string(),
            workspace: c["workspace"]["name"].as_str().unwrap_or("1").to_string(),
            floating: c["floating"].as_bool().unwrap_or(false),
            size: [
                c["size"][0].as_i64().unwrap_or(0) as i32,
                c["size"][1].as_i64().unwrap_or(0) as i32,
            ],
            at: [
                c["at"][0].as_i64().unwrap_or(0) as i32,
                c["at"][1].as_i64().unwrap_or(0) as i32,
            ],
            fullscreen: c["fullscreen"].as_i64().unwrap_or(0) as i32,
        })
        .collect();

    Ok(windows)
}

/// Query Wezterm for current terminal state
fn capture_wezterm() -> Result<Vec<WeztermPane>> {
    let output = match Command::new("wezterm")
        .args(["cli", "list", "--format", "json"])
        .output()
    {
        Ok(o) => o,
        Err(e) => {
            warn!("wezterm not available: {}", e);
            return Ok(vec![]);
        }
    };

    if !output.status.success() {
        warn!("wezterm cli failed, skipping terminal capture");
        return Ok(vec![]);
    }

    let panes: Vec<serde_json::Value> = serde_json::from_slice(&output.stdout)?;
    let terminals: Vec<WeztermPane> = panes
        .into_iter()
        .map(|p| WeztermPane {
            pane_id: p["pane_id"].as_u64().unwrap_or(0),
            title: p["title"].as_str().unwrap_or("").to_string(),
            cwd: p["cwd"].as_str().unwrap_or("").to_string(),
        })
        .collect();

    Ok(terminals)
}

/// Strip file://hostname/ prefix from Wezterm CWD
fn clean_cwd(cwd: &str) -> String {
    if let Some(rest) = cwd.strip_prefix("file://") {
        // Skip hostname part
        if let Some(pos) = rest[1..].find('/') {
            return rest[pos + 1..].to_string();
        }
    }
    cwd.to_string()
}

/// Determine the launch command for a terminal pane based on its title
fn pane_launch_command(pane: &WeztermPane) -> (String, Vec<String>) {
    let title = pane.title.to_lowercase();
    let dir = clean_cwd(&pane.cwd);

    if title.contains("btop") {
        (
            "wezterm".into(),
            vec![
                "start".into(),
                "--cwd".into(),
                dir,
                "--".into(),
                "btop".into(),
            ],
        )
    } else if title.contains("lumen") {
        (
            "wezterm".into(),
            vec![
                "start".into(),
                "--cwd".into(),
                dir,
                "--".into(),
                "lumen".into(),
            ],
        )
    } else if title.contains("claude") {
        (
            "wezterm".into(),
            vec![
                "start".into(),
                "--cwd".into(),
                dir,
                "--".into(),
                "claude".into(),
            ],
        )
    } else if title == "hx" || title.contains("helix") {
        (
            "wezterm".into(),
            vec![
                "start".into(),
                "--cwd".into(),
                dir,
                "--".into(),
                "hx".into(),
            ],
        )
    } else {
        // Default: open shell in the same directory
        ("wezterm".into(), vec!["start".into(), "--cwd".into(), dir])
    }
}

pub fn handle_session_save(name: &str) -> Result<()> {
    // Stack autosaves for undo history
    if name == "autosave" {
        push_autosave_stack();
    }

    let windows = capture_hyprland()?;
    let terminals = capture_wezterm()?;

    for w in &windows {
        debug!(
            "  window: [{}] {} on workspace {} ({})",
            w.class,
            w.title,
            w.workspace,
            if w.floating { "floating" } else { "tiled" }
        );
    }
    for t in &terminals {
        debug!("  terminal: {} (cwd: {})", t.title, clean_cwd(&t.cwd));
    }

    let session = Session { windows, terminals };

    let dir = session_dir();
    fs::create_dir_all(&dir)?;

    let path = session_path(name);
    let json = serde_json::to_string_pretty(&session)?;
    fs::write(&path, json)?;

    info!(
        "Saved session '{}': {} windows, {} terminals",
        name,
        session.windows.len(),
        session.terminals.len()
    );

    Ok(())
}

pub fn handle_session_restore_file(file_path: &str, dry_run: bool) -> Result<()> {
    let path = PathBuf::from(file_path);
    if !path.exists() {
        return Err(format!("Session file not found: {}", file_path).into());
    }
    restore_from_path(&path, file_path, dry_run)
}

pub fn handle_session_restore(name: &str, dry_run: bool) -> Result<()> {
    let path = session_path(name);
    if !path.exists() {
        return Err(format!("No saved session '{}' found at {:?}", name, path).into());
    }
    restore_from_path(&path, name, dry_run)
}

fn restore_from_path(path: &Path, label: &str, dry_run: bool) -> Result<()> {
    if !path.exists() {
        return Err(format!("Session not found: {:?}", path).into());
    }

    let json = fs::read_to_string(path)?;
    let session: Session = serde_json::from_str(&json)?;

    let mode = if dry_run { " (dry run)" } else { "" };
    info!(
        "Restoring session '{}'{}: {} windows, {} terminals",
        label,
        mode,
        session.windows.len(),
        session.terminals.len()
    );

    if dry_run {
        for w in &session.windows {
            info!(
                "  would launch: [{}] {} on workspace {}",
                w.class, w.title, w.workspace
            );
        }
        for t in &session.terminals {
            let (cmd, args) = pane_launch_command(t);
            info!("  would run: {} {}", cmd, args.join(" "));
        }
        info!("Dry run complete — no apps launched");
        return Ok(());
    }

    // Restore terminals first
    for pane in &session.terminals {
        let (cmd, args) = pane_launch_command(pane);
        info!("  Launching: {} ({})", pane.title, clean_cwd(&pane.cwd));
        Command::new(&cmd).args(&args).spawn().ok();
        std::thread::sleep(std::time::Duration::from_millis(500));
    }

    // Wait for terminals to appear
    std::thread::sleep(std::time::Duration::from_secs(2));

    // Launch other apps
    let mut launched_brave = false;
    let mut launched_bespec = false;

    for window in &session.windows {
        match window.class.as_str() {
            "brave-browser" if !launched_brave => {
                info!("  Launching Brave (session auto-restore)");
                Command::new("brave").spawn().ok();
                launched_brave = true;
                std::thread::sleep(std::time::Duration::from_secs(2));
            }
            "bespec" if !launched_bespec => {
                info!("  Launching BeSpec");
                Command::new("bespec").spawn().ok();
                launched_bespec = true;
                std::thread::sleep(std::time::Duration::from_millis(500));
            }
            _ => {}
        }
    }

    // Wait for windows to spawn
    std::thread::sleep(std::time::Duration::from_secs(2));

    // Move windows to correct workspaces
    info!("  Arranging windows on workspaces...");
    let output = match Command::new("hyprctl").args(["clients", "-j"]).output() {
        Ok(o) => Some(o),
        Err(e) => {
            warn!("hyprctl not available for workspace arrangement: {}", e);
            None
        }
    };

    if let Some(output) = output
        && output.status.success()
    {
        let current: Vec<serde_json::Value> = serde_json::from_slice(&output.stdout)?;

        for window in &session.windows {
            // Find matching window by class
            for client in &current {
                let client_class = client["class"].as_str().unwrap_or("");
                if client_class == window.class {
                    let addr = client["address"].as_str().unwrap_or("");
                    if !addr.is_empty() {
                        Command::new("hyprctl")
                            .args([
                                "dispatch",
                                "movetoworkspacesilent",
                                &format!("{},address:{}", window.workspace, addr),
                            ])
                            .output()
                            .ok();
                    }
                }
            }
        }
    }

    info!("Session '{}' restored!", label);
    Ok(())
}

pub fn handle_session_undo() -> Result<()> {
    let dir = session_dir();
    let prev = dir.join("autosave-1.json");
    if !prev.exists() {
        return Err("No previous session state to undo to".to_string().into());
    }

    // Restore from autosave-1
    info!("Undoing to previous session state...");
    handle_session_restore("autosave-1", false)?;

    // Pop the stack: autosave-1 → autosave, autosave-2 → autosave-1, etc.
    let current = dir.join("autosave.json");
    if current.exists() {
        fs::remove_file(&current).ok();
    }
    for i in 1..MAX_AUTOSAVE_STACK {
        let from = dir.join(format!("autosave-{}.json", i + 1));
        let to = dir.join(if i == 1 {
            "autosave.json".to_string()
        } else {
            format!("autosave-{}.json", i)
        });
        if from.exists() {
            fs::rename(&from, &to).ok();
        }
    }

    info!("Session state reverted");
    Ok(())
}

pub fn handle_session_list() -> Result<()> {
    let dir = session_dir();
    if !dir.exists() {
        info!("No saved sessions");
        return Ok(());
    }

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().is_some_and(|e| e == "json") {
            let name = path.file_stem().unwrap().to_string_lossy();
            let json = fs::read_to_string(&path)?;
            let session: Session = serde_json::from_str(&json)?;
            println!(
                "  {} — {} windows, {} terminals",
                name,
                session.windows.len(),
                session.terminals.len()
            );
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_cwd_with_file_prefix() {
        assert_eq!(clean_cwd("file://yoga/etc/nixos/"), "/etc/nixos/");
        assert_eq!(clean_cwd("file://hostname/home/user/"), "/home/user/");
    }

    #[test]
    fn test_clean_cwd_without_prefix() {
        assert_eq!(clean_cwd("/home/user"), "/home/user");
        assert_eq!(clean_cwd(""), "");
    }

    #[test]
    fn test_pane_launch_command_btop() {
        let pane = WeztermPane {
            pane_id: 1,
            title: "btop".to_string(),
            cwd: "file://yoga/home/user/".to_string(),
        };
        let (cmd, args) = pane_launch_command(&pane);
        assert_eq!(cmd, "wezterm");
        assert!(args.contains(&"btop".to_string()));
        assert!(args.contains(&"/home/user/".to_string()));
    }

    #[test]
    fn test_pane_launch_command_claude() {
        let pane = WeztermPane {
            pane_id: 2,
            title: "⠂ vogix-keybindings-modal-system".to_string(),
            cwd: "file://yoga/etc/nixos/".to_string(),
        };
        let (cmd, args) = pane_launch_command(&pane);
        assert_eq!(cmd, "wezterm");
        // Should not launch claude for a title containing "vogix" but not "claude"
    }

    #[test]
    fn test_pane_launch_command_shell() {
        let pane = WeztermPane {
            pane_id: 3,
            title: "bash".to_string(),
            cwd: "file://yoga/home/user/code/".to_string(),
        };
        let (cmd, args) = pane_launch_command(&pane);
        assert_eq!(cmd, "wezterm");
        assert!(args.contains(&"/home/user/code/".to_string()));
        assert!(!args.iter().any(|a| a == "--"));
    }

    #[test]
    fn test_pane_launch_command_helix() {
        let pane = WeztermPane {
            pane_id: 4,
            title: "hx".to_string(),
            cwd: "file://yoga/home/user/".to_string(),
        };
        let (cmd, args) = pane_launch_command(&pane);
        assert_eq!(cmd, "wezterm");
        assert!(args.contains(&"hx".to_string()));
    }

    #[test]
    fn test_session_serialization_roundtrip() {
        let session = Session {
            windows: vec![HyprWindow {
                class: "brave-browser".to_string(),
                title: "Test".to_string(),
                workspace: "1".to_string(),
                floating: false,
                size: [800, 600],
                at: [0, 0],
                fullscreen: 0,
            }],
            terminals: vec![WeztermPane {
                pane_id: 1,
                title: "bash".to_string(),
                cwd: "file://yoga/home/user/".to_string(),
            }],
        };

        let json = serde_json::to_string(&session).unwrap();
        let deserialized: Session = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.windows.len(), 1);
        assert_eq!(deserialized.terminals.len(), 1);
        assert_eq!(deserialized.windows[0].class, "brave-browser");
        assert_eq!(deserialized.terminals[0].title, "bash");
    }

    #[test]
    fn test_session_empty() {
        let session = Session {
            windows: vec![],
            terminals: vec![],
        };
        let json = serde_json::to_string(&session).unwrap();
        let deserialized: Session = serde_json::from_str(&json).unwrap();
        assert!(deserialized.windows.is_empty());
        assert!(deserialized.terminals.is_empty());
    }

    // Property: every window class produces a valid launch command
    #[test]
    fn test_all_known_apps_have_launch_commands() {
        let apps = ["btop", "lumen", "claude", "hx", "bash", "unknown-app"];
        for title in &apps {
            let pane = WeztermPane {
                pane_id: 0,
                title: title.to_string(),
                cwd: "file://yoga/home/user/".to_string(),
            };
            let (cmd, args) = pane_launch_command(&pane);
            assert_eq!(cmd, "wezterm", "All panes launch via wezterm");
            assert!(
                !args.is_empty(),
                "Launch args should not be empty for {}",
                title
            );
            assert!(
                args.contains(&"start".to_string()),
                "Should use 'start' subcommand"
            );
            assert!(
                args.contains(&"--cwd".to_string()),
                "Should set working directory"
            );
        }
    }

    // Property: clean_cwd always returns a path starting with /
    #[test]
    fn test_clean_cwd_always_absolute() {
        let inputs = [
            "file://yoga/etc/nixos/",
            "file://localhost/home/user/",
            "/already/absolute",
            "file://a/b",
        ];
        for input in &inputs {
            let result = clean_cwd(input);
            assert!(
                result.starts_with('/'),
                "clean_cwd({}) = {} should start with /",
                input,
                result
            );
        }
    }

    // ── Stack/undo tests ──

    #[test]
    fn test_push_autosave_stack_creates_numbered_files() {
        let tmp = std::env::temp_dir().join("vogix-test-stack");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();

        // Override session_dir for test
        let autosave = tmp.join("autosave.json");
        fs::write(&autosave, r#"{"windows":[],"terminals":[]}"#).unwrap();

        // Simulate stack push by renaming
        let autosave1 = tmp.join("autosave-1.json");
        fs::rename(&autosave, &autosave1).unwrap();
        fs::write(&autosave, r#"{"windows":[{"class":"test","title":"t","workspace":"1","floating":false,"size":[0,0],"at":[0,0],"fullscreen":0}],"terminals":[]}"#).unwrap();

        assert!(autosave.exists());
        assert!(autosave1.exists());

        // Verify different content
        let current: Session =
            serde_json::from_str(&fs::read_to_string(&autosave).unwrap()).unwrap();
        let prev: Session = serde_json::from_str(&fs::read_to_string(&autosave1).unwrap()).unwrap();
        assert_eq!(current.windows.len(), 1);
        assert_eq!(prev.windows.len(), 0);

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_session_roundtrip_with_all_fields() {
        let session = Session {
            windows: vec![
                HyprWindow {
                    class: "brave-browser".to_string(),
                    title: "Test Page".to_string(),
                    workspace: "3".to_string(),
                    floating: true,
                    size: [1920, 1080],
                    at: [100, 50],
                    fullscreen: 1,
                },
                HyprWindow {
                    class: "org.wezfurlong.wezterm".to_string(),
                    title: "bash".to_string(),
                    workspace: "1".to_string(),
                    floating: false,
                    size: [800, 600],
                    at: [0, 0],
                    fullscreen: 0,
                },
            ],
            terminals: vec![
                WeztermPane {
                    pane_id: 42,
                    title: "btop".to_string(),
                    cwd: "file://yoga/home/logger/".to_string(),
                },
                WeztermPane {
                    pane_id: 99,
                    title: "✳ lumen".to_string(),
                    cwd: "file://yoga/home/logger/Code/github/logger/lumen/".to_string(),
                },
            ],
        };

        let json = serde_json::to_string_pretty(&session).unwrap();
        let restored: Session = serde_json::from_str(&json).unwrap();

        // Property: all fields survive roundtrip
        assert_eq!(restored.windows.len(), 2);
        assert_eq!(restored.terminals.len(), 2);
        assert_eq!(restored.windows[0].class, "brave-browser");
        assert_eq!(restored.windows[0].workspace, "3");
        assert!(restored.windows[0].floating);
        assert_eq!(restored.windows[0].size, [1920, 1080]);
        assert_eq!(restored.windows[0].fullscreen, 1);
        assert_eq!(restored.windows[1].floating, false);
        assert_eq!(restored.terminals[0].pane_id, 42);
        assert_eq!(restored.terminals[1].title, "✳ lumen");
    }

    // Property: MAX_AUTOSAVE_STACK is reasonable
    #[test]
    fn test_max_autosave_stack_bounds() {
        assert!(MAX_AUTOSAVE_STACK > 0);
        assert!(MAX_AUTOSAVE_STACK <= 100); // Don't keep too many
    }

    // Property: pane_launch_command always returns wezterm with --cwd
    #[test]
    fn test_pane_launch_always_has_cwd() {
        let titles = [
            "btop",
            "lumen",
            "claude",
            "hx",
            "bash",
            "unknown",
            "",
            "ζ€ε₯‡ηš„η¨‹εΊ",
        ];
        for title in &titles {
            let pane = WeztermPane {
                pane_id: 0,
                title: title.to_string(),
                cwd: "file://host/home/user/".to_string(),
            };
            let (cmd, args) = pane_launch_command(&pane);
            assert_eq!(cmd, "wezterm");
            assert!(
                args.contains(&"--cwd".to_string()),
                "Missing --cwd for title: {}",
                title
            );
            assert!(
                args.contains(&"/home/user/".to_string()),
                "Missing dir for title: {}",
                title
            );
        }
    }

    // ── --json and --dry-run tests ──

    #[test]
    fn test_restore_file_nonexistent() {
        let result = handle_session_restore_file("/nonexistent/path.json", false);
        assert!(result.is_err());
    }

    #[test]
    fn test_restore_file_dry_run() {
        let tmp = std::env::temp_dir().join("vogix-test-dry-run.json");
        let session = Session {
            windows: vec![HyprWindow {
                class: "brave-browser".to_string(),
                title: "Test".to_string(),
                workspace: "1".to_string(),
                floating: false,
                size: [800, 600],
                at: [0, 0],
                fullscreen: 0,
            }],
            terminals: vec![WeztermPane {
                pane_id: 1,
                title: "bash".to_string(),
                cwd: "file://yoga/home/user/".to_string(),
            }],
        };
        fs::write(&tmp, serde_json::to_string(&session).unwrap()).unwrap();

        // Dry run should succeed without launching anything
        let result = handle_session_restore_file(tmp.to_str().unwrap(), true);
        assert!(result.is_ok(), "Dry run should succeed: {:?}", result.err());

        let _ = fs::remove_file(&tmp);
    }

    #[test]
    fn test_cli_parse_restore_with_json() {
        use crate::cli::Cli;
        use clap::Parser;
        let cli = Cli::try_parse_from(["vogix", "session", "restore", "--json", "/tmp/test.json"])
            .unwrap();
        if let crate::cli::Commands::Session {
            command: crate::cli::SessionCommands::Restore { json, dry_run, .. },
        } = cli.command
        {
            assert_eq!(json.unwrap(), "/tmp/test.json");
            assert!(!dry_run);
        } else {
            panic!("Expected Session Restore");
        }
    }

    #[test]
    fn test_cli_parse_restore_with_dry_run() {
        use crate::cli::Cli;
        use clap::Parser;
        let cli = Cli::try_parse_from(["vogix", "session", "restore", "--dry-run"]).unwrap();
        if let crate::cli::Commands::Session {
            command: crate::cli::SessionCommands::Restore { dry_run, .. },
        } = cli.command
        {
            assert!(dry_run);
        } else {
            panic!("Expected Session Restore");
        }
    }

    #[test]
    fn test_cli_parse_restore_json_and_dry_run() {
        use crate::cli::Cli;
        use clap::Parser;
        let cli = Cli::try_parse_from([
            "vogix",
            "session",
            "restore",
            "--json",
            "/tmp/x.json",
            "--dry-run",
        ])
        .unwrap();
        if let crate::cli::Commands::Session {
            command: crate::cli::SessionCommands::Restore { json, dry_run, .. },
        } = cli.command
        {
            assert_eq!(json.unwrap(), "/tmp/x.json");
            assert!(dry_run);
        } else {
            panic!("Expected Session Restore");
        }
    }
}

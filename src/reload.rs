use crate::config::Config;
use crate::errors::{Result, VogixError};
use log::{debug, warn};
use std::collections::HashMap;
use std::process::Command;

/// What reloading one application did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReloadOutcome {
    /// The application was told to reload; the text says how.
    Reloaded(String),
    /// The application's process is not running, so there is nothing to
    /// reload: it reads the new theme when it starts.
    NotRunning,
    /// `reload_method = "none"`: the theme takes effect on next launch.
    NotNeeded,
}

/// Result of reloading applications
#[derive(Debug)]
pub struct ReloadResult {
    /// Number of apps successfully reloaded
    pub success_count: usize,
    /// Apps that attempted a reload (excludes apps with reload_method = "none"
    /// and apps that are not running)
    pub total_count: usize,
    /// Apps that failed to reload with error messages
    pub failed_apps: Vec<(String, String)>,
    /// Apps with a signal reload whose process is not running
    pub not_running: Vec<String>,
}

impl ReloadResult {
    /// Returns true if any applications failed to reload
    pub fn has_failures(&self) -> bool {
        !self.failed_apps.is_empty()
    }
}

pub struct ReloadDispatcher;

impl ReloadDispatcher {
    pub fn new() -> Self {
        ReloadDispatcher
    }

    /// Reload all themed applications
    /// Returns a ReloadResult with details about successes and failures.
    /// When `quiet` is true, suppresses success messages (errors still go to stderr).
    pub fn reload_apps(&self, config: &Config, quiet: bool) -> ReloadResult {
        if config.apps.is_empty() {
            if !quiet {
                println!("No applications configured");
            }
            return ReloadResult {
                success_count: 0,
                total_count: 0,
                failed_apps: Vec::new(),
                not_running: Vec::new(),
            };
        }

        let mut failed_apps = Vec::new();
        let mut not_running = Vec::new();
        let mut success_count = 0;

        for (app_name, app_metadata) in &config.apps {
            match self.reload_app(app_name, app_metadata) {
                Ok(ReloadOutcome::Reloaded(how)) => {
                    debug!("{app_name}: {how}");
                    success_count += 1;
                }
                Ok(ReloadOutcome::NotRunning) => not_running.push(app_name.clone()),
                Ok(ReloadOutcome::NotNeeded) => {}
                Err(e) => failed_apps.push((app_name.clone(), e.to_string())),
            }
        }
        not_running.sort();

        let total_count = success_count + failed_apps.len();

        if failed_apps.is_empty() {
            if !quiet {
                if total_count > 0 {
                    println!("✓ Reloaded {} applications", success_count);
                } else {
                    println!("No applications needed reloading");
                }
            }
        } else {
            // Always show errors, even in quiet mode
            eprintln!(
                "⚠ Reloaded {}/{} applications. Failures:",
                success_count, total_count
            );
            for (app_name, error) in &failed_apps {
                eprintln!("  - {}: {}", app_name, error);
            }
        }

        ReloadResult {
            success_count,
            total_count,
            failed_apps,
            not_running,
        }
    }

    /// Reload a single application using metadata from manifest
    fn reload_app(
        &self,
        app_name: &str,
        metadata: &crate::config::AppMetadata,
    ) -> Result<ReloadOutcome> {
        match metadata.reload_method.as_str() {
            "signal" => {
                let signal = metadata.reload_signal.as_ref().ok_or_else(|| {
                    VogixError::reload("signal reload method requires reload_signal")
                })?;
                let process_name = metadata.process_name.as_deref().unwrap_or(app_name);
                self.send_signal(process_name, signal)
            }
            "command" => {
                let cmd = metadata.reload_command.as_ref().ok_or_else(|| {
                    VogixError::reload("command reload method requires reload_command")
                })?;
                self.run_command(cmd)?;
                Ok(ReloadOutcome::Reloaded(
                    "executed reload command".to_string(),
                ))
            }
            "touch" => {
                self.touch_or_relink(&metadata.config_path)?;
                if let Some(theme_path) = &metadata.theme_file_path {
                    let _ = self.touch_or_relink(theme_path);
                }
                Ok(ReloadOutcome::Reloaded(
                    "touched to trigger auto-reload".to_string(),
                ))
            }
            "none" => Ok(ReloadOutcome::NotNeeded),
            _ => Err(VogixError::reload(format!(
                "unknown reload method: {}",
                metadata.reload_method
            ))),
        }
    }

    /// Touch a file or re-create a symlink to trigger directory-level inotify events.
    /// Symlinks are removed and re-created so watchers on the parent directory
    /// see Create/Remove events (touch -h only changes symlink mtime which
    /// inotify directory watchers don't detect).
    /// Touch or re-symlink a config file to trigger inotify events.
    /// Note: uses unix-specific symlink API (vogix is NixOS/Linux-only).
    fn touch_or_relink(&self, path: &str) -> Result<()> {
        let p = std::path::Path::new(path);
        if p.is_symlink() {
            let target = std::fs::read_link(p)
                .map_err(|e| VogixError::reload_with_source("failed to read symlink", e))?;
            std::fs::remove_file(p)
                .map_err(|e| VogixError::reload_with_source("failed to remove symlink", e))?;
            std::os::unix::fs::symlink(&target, p)
                .map_err(|e| VogixError::reload_with_source("failed to recreate symlink", e))?;
        } else {
            let status = Command::new("touch")
                .arg(path)
                .status()
                .map_err(|e| VogixError::reload_with_source("failed to touch config file", e))?;
            if !status.success() {
                return Err(VogixError::reload(format!(
                    "touch {path} failed with exit code {:?}",
                    status.code()
                )));
            }
        }
        Ok(())
    }

    /// Send a Unix signal to every process named `process_name` (its
    /// `/proc/<pid>/comm`). No such process is [`ReloadOutcome::NotRunning`];
    /// processes that exist but could not be signalled are an error.
    fn send_signal(&self, process_name: &str, signal: &str) -> Result<ReloadOutcome> {
        use std::fs;

        let sig = match signal.trim_start_matches("SIG") {
            "USR1" => libc::SIGUSR1,
            "USR2" => libc::SIGUSR2,
            "HUP" => libc::SIGHUP,
            "TERM" => libc::SIGTERM,
            "INT" => libc::SIGINT,
            s => {
                return Err(VogixError::reload(format!("unsupported signal: {}", s)));
            }
        };

        let mut matched = 0usize;
        let mut signalled = 0usize;
        if let Ok(entries) = fs::read_dir("/proc") {
            for entry in entries.flatten() {
                let name = entry.file_name();
                let pid_str = name.to_string_lossy();

                // Only numeric directories are PIDs
                if !pid_str.chars().all(|c| c.is_ascii_digit()) {
                    continue;
                }

                // Read /proc/{pid}/comm for the process name
                let comm_path = entry.path().join("comm");
                if let Ok(comm) = fs::read_to_string(&comm_path)
                    && comm.trim() == process_name
                    && let Ok(pid) = pid_str.parse::<i32>()
                {
                    matched += 1;
                    // SAFETY: kill(2) with a pid read from /proc and a valid
                    // signal number; it has no memory-safety preconditions.
                    let ret = unsafe { libc::kill(pid, sig) };
                    if ret == 0 {
                        signalled += 1;
                    } else {
                        warn!(
                            "Signal {} to {} (pid {}) failed: {}",
                            sig,
                            process_name,
                            pid,
                            std::io::Error::last_os_error()
                        );
                    }
                }
            }
        }

        signal_outcome(process_name, signal, matched, signalled)
    }

    /// Run a shell command
    fn run_command(&self, cmd: &str) -> Result<()> {
        let output = Command::new("sh")
            .arg("-c")
            .arg(cmd)
            .output()
            .map_err(|e| VogixError::reload_with_source("failed to run command", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(VogixError::reload(format!("command failed: {}", stderr)));
        }

        Ok(())
    }

    /// Run the user apply hooks, all in parallel.
    ///
    /// Each hook runs as its own `sh -c` child with every `{{slot}}`
    /// placeholder replaced by that slot's colour without the '#'; the call
    /// returns once every child has exited. A hook that cannot start or
    /// exits non-zero is reported and does not fail the apply.
    pub fn run_apply_hooks(&self, config: &Config, colors: &HashMap<String, String>, quiet: bool) {
        if config.hooks.is_empty() {
            return;
        }

        let children: Vec<(&str, std::io::Result<std::process::Child>)> = config
            .hooks
            .iter()
            .map(|(name, hook)| {
                let command = substitute_slots(&hook.command, colors);
                let child = Command::new("sh")
                    .arg("-c")
                    .arg(&command)
                    .stdout(std::process::Stdio::piped())
                    .stderr(std::process::Stdio::piped())
                    .spawn();
                (name.as_str(), child)
            })
            .collect();

        for (name, child) in children {
            let child = match child {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("⚠ Hook {}: failed to spawn: {}", name, e);
                    continue;
                }
            };

            match child.wait_with_output() {
                Ok(out) if out.status.success() => {
                    if !quiet {
                        println!("✓ Hook: {}", name);
                    }
                }
                Ok(out) => {
                    let stderr = String::from_utf8_lossy(&out.stderr);
                    eprintln!("⚠ Hook {}: command failed: {}", name, stderr.trim());
                }
                Err(e) => eprintln!("⚠ Hook {}: {}", name, e),
            }
        }
    }
}

/// What a signal reload did, from how many processes carried the name and
/// how many of them the signal reached.
fn signal_outcome(
    process_name: &str,
    signal: &str,
    matched: usize,
    signalled: usize,
) -> Result<ReloadOutcome> {
    match (matched, signalled) {
        (0, _) => Ok(ReloadOutcome::NotRunning),
        (_, 0) => Err(VogixError::reload(format!(
            "no '{process_name}' process accepted SIG{}",
            signal.trim_start_matches("SIG")
        ))),
        _ => Ok(ReloadOutcome::Reloaded(format!("sent {signal} signal"))),
    }
}

/// `command` with every `{{slot}}` placeholder of a known slot replaced by
/// that slot's colour, leading '#' removed.
fn substitute_slots(command: &str, colors: &HashMap<String, String>) -> String {
    let mut command = command.to_string();
    for (slot, hex) in colors {
        command = command.replace(&format!("{{{{{slot}}}}}"), hex.trim_start_matches('#'));
    }
    command
}

impl Default for ReloadDispatcher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{AppMetadata, ApplyHook};
    use std::collections::BTreeMap;

    fn app(reload_method: &str) -> AppMetadata {
        AppMetadata {
            config_path: "/tmp/test.conf".to_string(),
            reload_method: reload_method.to_string(),
            reload_signal: None,
            process_name: None,
            reload_command: None,
            theme_file_path: None,
        }
    }

    fn config_with(
        apps: HashMap<String, AppMetadata>,
        hooks: BTreeMap<String, ApplyHook>,
    ) -> Config {
        Config {
            default_theme: "test".to_string(),
            default_variant: "dark".to_string(),
            default_scheme: crate::scheme::Scheme::Vogix16,
            apps,
            hooks,
            templates: None,
            theme_sources: None,
            shader: None,
        }
    }

    /// A process name no test host runs.
    const ABSENT_PROCESS: &str = "vogix-absent-t";

    #[test]
    fn test_reload_app_with_touch_method() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("app.conf");
        std::fs::write(&path, "x").unwrap();
        let metadata = AppMetadata {
            config_path: path.display().to_string(),
            ..app("touch")
        };
        match ReloadDispatcher::new().reload_app("test", &metadata) {
            Ok(ReloadOutcome::Reloaded(how)) => assert!(how.contains("touched"), "{how}"),
            other => panic!("expected a touch reload, got {other:?}"),
        }
    }

    #[test]
    fn test_reload_app_with_none_method() {
        let result = ReloadDispatcher::new().reload_app("test", &app("none"));
        assert_eq!(result.unwrap(), ReloadOutcome::NotNeeded);
    }

    #[test]
    fn a_signal_reload_of_an_app_that_is_not_running_is_not_running() {
        let metadata = AppMetadata {
            reload_signal: Some("SIGUSR1".to_string()),
            process_name: Some(ABSENT_PROCESS.to_string()),
            ..app("signal")
        };
        let result = ReloadDispatcher::new().reload_app("btop", &metadata);
        assert_eq!(result.unwrap(), ReloadOutcome::NotRunning);
    }

    #[test]
    fn an_app_that_is_not_running_is_not_a_failure() {
        let mut apps = HashMap::new();
        apps.insert(
            "btop".to_string(),
            AppMetadata {
                reload_signal: Some("USR1".to_string()),
                process_name: Some(ABSENT_PROCESS.to_string()),
                ..app("signal")
            },
        );
        apps.insert("alacritty".to_string(), app("none"));
        let result = ReloadDispatcher::new().reload_apps(&config_with(apps, BTreeMap::new()), true);
        assert!(!result.has_failures(), "{:?}", result.failed_apps);
        assert_eq!(result.not_running, ["btop"]);
        assert_eq!(result.total_count, 0);
        assert_eq!(result.success_count, 0);
    }

    #[test]
    fn an_unsupported_signal_is_an_error() {
        let metadata = AppMetadata {
            reload_signal: Some("SIGWINCH".to_string()),
            process_name: Some(ABSENT_PROCESS.to_string()),
            ..app("signal")
        };
        let err = ReloadDispatcher::new()
            .reload_app("x", &metadata)
            .unwrap_err()
            .to_string();
        assert!(err.contains("unsupported signal: WINCH"), "{err}");
    }

    #[test]
    fn the_signal_outcome_follows_matches_and_deliveries() {
        assert_eq!(
            signal_outcome("btop", "SIGUSR1", 0, 0).unwrap(),
            ReloadOutcome::NotRunning
        );
        let err = signal_outcome("btop", "SIGUSR1", 2, 0)
            .unwrap_err()
            .to_string();
        assert!(err.contains("no 'btop' process accepted SIGUSR1"), "{err}");
        assert_eq!(
            signal_outcome("btop", "USR1", 2, 1).unwrap(),
            ReloadOutcome::Reloaded("sent USR1 signal".to_string())
        );
    }

    #[test]
    fn test_reload_apps_returns_failure_count() {
        let mut apps = HashMap::new();
        apps.insert(
            "failing_app".to_string(),
            AppMetadata {
                reload_command: Some("exit 1".to_string()),
                ..app("command")
            },
        );
        apps.insert("skipped_app".to_string(), app("none"));
        let result =
            ReloadDispatcher::new().reload_apps(&config_with(apps, BTreeMap::new()), false);
        assert!(
            result.has_failures(),
            "reload_apps should report failures when apps fail to reload"
        );
        assert_eq!(result.total_count, 1);
    }

    #[test]
    fn hooks_run_with_their_slots_substituted_without_the_hash() {
        let dir = tempfile::tempdir().unwrap();
        let out = |name: &str| dir.path().join(name);
        let mut hooks = BTreeMap::new();
        for name in ["first", "second"] {
            hooks.insert(
                name.to_string(),
                ApplyHook {
                    command: format!(
                        "printf '%s %s' {{{{base00}}}} {{{{base01}}}} > {}",
                        out(name).display()
                    ),
                },
            );
        }
        hooks.insert(
            "failing".to_string(),
            ApplyHook {
                command: "exit 3".to_string(),
            },
        );
        let mut colors = HashMap::new();
        colors.insert("base00".to_string(), "#ff0000".to_string());
        colors.insert("base01".to_string(), "#333333".to_string());

        ReloadDispatcher::new().run_apply_hooks(&config_with(HashMap::new(), hooks), &colors, true);
        for name in ["first", "second"] {
            assert_eq!(std::fs::read_to_string(out(name)).unwrap(), "ff0000 333333");
        }
    }

    #[test]
    fn placeholders_of_unknown_slots_are_left_alone() {
        let mut colors = HashMap::new();
        colors.insert("base00".to_string(), "#262626".to_string());
        assert_eq!(
            substitute_slots("x {{base00}} {{nope}} {{base00}}", &colors),
            "x 262626 {{nope}} 262626"
        );
    }
}

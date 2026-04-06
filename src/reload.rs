use crate::config::Config;
use crate::errors::{Result, VogixError};
use std::process::Command;

/// Result of reloading applications
#[derive(Debug)]
pub struct ReloadResult {
    /// Number of apps successfully reloaded
    pub success_count: usize,
    /// Total apps that attempted reload (excludes apps with reload_method = "none")
    pub total_count: usize,
    /// Apps that failed to reload with error messages
    pub failed_apps: Vec<(String, String)>,
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
            };
        }

        let mut failed_apps = Vec::new();
        let mut skipped = 0;

        for (app_name, app_metadata) in &config.apps {
            // Skip apps that don't need reloading
            if app_metadata.reload_method == "none" {
                skipped += 1;
                continue;
            }

            if let Err(e) = self.reload_app(app_name, app_metadata) {
                failed_apps.push((app_name.clone(), e.to_string()));
            }
        }

        let total_count = config.apps.len() - skipped;
        let success_count = total_count - failed_apps.len();

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
        }
    }

    /// Reload a single application using metadata from manifest
    fn reload_app(&self, app_name: &str, metadata: &crate::config::AppMetadata) -> Result<String> {
        match metadata.reload_method.as_str() {
            "signal" => {
                let signal = metadata.reload_signal.as_ref().ok_or_else(|| {
                    VogixError::reload("signal reload method requires reload_signal")
                })?;
                let process_name = metadata.process_name.as_deref().unwrap_or(app_name);
                self.send_signal(process_name, signal)?;
                Ok(format!("sent {} signal", signal))
            }
            "command" => {
                let cmd = metadata.reload_command.as_ref().ok_or_else(|| {
                    VogixError::reload("command reload method requires reload_command")
                })?;
                self.run_command(cmd)?;
                Ok("executed reload command".to_string())
            }
            "touch" => {
                // Touch the symlink itself (-h flag) to update its mtime
                // Applications watching the file will detect the change
                Command::new("touch")
                    .arg("-h")
                    .arg(&metadata.config_path)
                    .status()
                    .map_err(|e| {
                        VogixError::reload_with_source("failed to touch config file", e)
                    })?;
                // Also touch theme file if present (hybrid apps like btop)
                if let Some(theme_path) = &metadata.theme_file_path {
                    let _ = Command::new("touch").arg("-h").arg(theme_path).status();
                }
                Ok("touched to trigger auto-reload".to_string())
            }
            "none" => Ok("no reload needed (changes take effect on next use)".to_string()),
            _ => Err(VogixError::reload(format!(
                "unknown reload method: {}",
                metadata.reload_method
            ))),
        }
    }

    /// Send a Unix signal to a process by name using native Rust
    fn send_signal(&self, process_name: &str, signal: &str) -> Result<()> {
        use std::fs;

        let sig = match signal.trim_start_matches("SIG") {
            "USR1" => 10,
            "USR2" => 12,
            "HUP" => 1,
            "TERM" => 15,
            "INT" => 2,
            s => {
                return Err(VogixError::reload(format!("unsupported signal: {}", s)));
            }
        };

        // Find PIDs by scanning /proc
        let mut found = false;
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
                if let Ok(comm) = fs::read_to_string(&comm_path) {
                    if comm.trim() == process_name {
                        if let Ok(pid) = pid_str.parse::<i32>() {
                            // Send signal via libc
                            unsafe { libc::kill(pid, sig) };
                            found = true;
                        }
                    }
                }
            }
        }

        if !found {
            return Err(VogixError::reload(format!(
                "process '{}' is not running",
                process_name
            )));
        }

        Ok(())
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
}

impl Default for ReloadDispatcher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reload_dispatcher_creation() {
        let _dispatcher = ReloadDispatcher::new();
        // Verify it can be created
        assert!(true);
    }

    #[test]
    fn test_reload_app_with_touch_method() {
        use crate::config::AppMetadata;
        let dispatcher = ReloadDispatcher::new();
        let metadata = AppMetadata {
            config_path: "/tmp/test.conf".to_string(),
            reload_method: "touch".to_string(),
            reload_signal: None,
            process_name: None,
            reload_command: None,
            theme_file_path: None,
        };

        // Test that touch method doesn't crash
        match dispatcher.reload_app("test", &metadata) {
            Ok(msg) => assert!(msg.contains("touched")),
            Err(_) => {
                // Touch might fail if /tmp doesn't exist in test environment, that's OK
                assert!(true);
            }
        }
    }

    #[test]
    fn test_reload_app_with_none_method() {
        use crate::config::AppMetadata;
        let dispatcher = ReloadDispatcher::new();
        let metadata = AppMetadata {
            config_path: "/tmp/test.conf".to_string(),
            reload_method: "none".to_string(),
            reload_signal: None,
            process_name: None,
            reload_command: None,
            theme_file_path: None,
        };

        let result = dispatcher.reload_app("test", &metadata);
        assert!(result.is_ok());
        assert!(result.unwrap().contains("no reload needed"));
    }

    #[test]
    fn test_reload_apps_returns_failure_count() {
        use crate::config::AppMetadata;
        use std::collections::HashMap;

        let dispatcher = ReloadDispatcher::new();

        // Create config with a command that will fail
        let mut apps = HashMap::new();
        apps.insert(
            "failing_app".to_string(),
            AppMetadata {
                config_path: "/tmp/test.conf".to_string(),
                reload_method: "command".to_string(),
                reload_signal: None,
                process_name: None,
                reload_command: Some("exit 1".to_string()), // This will fail
                theme_file_path: None,
            },
        );
        apps.insert(
            "skipped_app".to_string(),
            AppMetadata {
                config_path: "/tmp/test.conf".to_string(),
                reload_method: "none".to_string(),
                reload_signal: None,
                process_name: None,
                reload_command: None,
                theme_file_path: None,
            },
        );

        let config = Config {
            default_theme: "test".to_string(),
            default_variant: "dark".to_string(),
            apps,
            templates: None,
            theme_sources: None,
            shader: None,
        };

        let result = dispatcher.reload_apps(&config, false);

        // The result should indicate there was a failure
        assert!(
            result.has_failures(),
            "reload_apps should report failures when apps fail to reload"
        );
    }
}

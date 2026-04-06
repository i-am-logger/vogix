use crate::scheme::Scheme;
use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser)]
#[command(name = env!("CARGO_PKG_NAME"))]
#[command(author = env!("CARGO_PKG_AUTHORS"))]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = env!("CARGO_PKG_DESCRIPTION"), long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Manage themes (colors, schemes, variants)
    Theme {
        #[command(subcommand)]
        command: ThemeCommands,
    },

    /// Manage desktop sessions (save/restore workspaces)
    Session {
        #[command(subcommand)]
        command: SessionCommands,
    },

    /// Generate shell completions
    Completions {
        /// Shell to generate completions for
        shell: CompletionShell,
    },

    /// Manage the template cache
    Cache {
        #[command(subcommand)]
        command: CacheCommands,
    },

    /// Toggle monochromatic screen shader
    Shader {
        #[command(subcommand)]
        command: ShaderCommands,
    },

    /// Run the vogix daemon (session auto-save, event monitoring)
    Daemon,
}

#[derive(Subcommand)]
pub enum ShaderCommands {
    /// Turn shader on (applies current theme's monochromatic tint)
    On,
    /// Turn shader off
    Off,
    /// Toggle shader on/off
    Toggle,
}

#[derive(Subcommand)]
pub enum ThemeCommands {
    /// List available themes
    #[command(alias = "ls")]
    List {
        /// Filter by scheme (vogix16, base16, base24, ansi16)
        #[arg(short = 's', long)]
        scheme: Option<Scheme>,

        /// Show variants for each theme
        #[arg(long)]
        variants: bool,
    },

    /// Show current theme status
    Status,

    /// Set theme, variant, or scheme
    Set {
        /// Color scheme (vogix16, base16, base24, ansi16)
        #[arg(short = 's', long)]
        scheme: Option<Scheme>,

        /// Theme name
        #[arg(short = 't', long)]
        theme: Option<String>,

        /// Variant (e.g., dark, light, dawn, moon, darker, lighter)
        #[arg(short = 'v', long)]
        variant: Option<String>,

        /// Suppress non-error output
        #[arg(short = 'q', long)]
        quiet: bool,
    },

    /// Refresh current theme (reapply without changes)
    Refresh {
        /// Suppress non-error output
        #[arg(short = 'q', long)]
        quiet: bool,
    },
}

#[derive(Subcommand)]
pub enum SessionCommands {
    /// Save current desktop session
    Save {
        /// Session name (default: "last")
        #[arg(default_value = "last")]
        name: String,
    },

    /// Restore a saved desktop session
    Restore {
        /// Session name (default: "last")
        #[arg(default_value = "last")]
        name: String,

        /// Restore from a JSON file path instead of a named session
        #[arg(long)]
        json: Option<String>,

        /// Validate and print session without launching apps
        #[arg(long)]
        dry_run: bool,
    },

    /// List saved sessions
    #[command(alias = "ls")]
    List,

    /// Undo last window change (restore from autosave stack)
    Undo,
}

#[derive(Subcommand)]
pub enum CacheCommands {
    /// Remove stale cache entries from old template versions
    Clean,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
pub enum CompletionShell {
    Bash,
    Zsh,
    Fish,
    Pwsh,
    Elvish,
}

impl Cli {
    pub fn parse_args() -> Self {
        Self::parse()
    }
}

/// Helper to check if a variant is a navigation command
pub fn is_variant_navigation(variant: &Option<String>) -> bool {
    if let Some(v) = variant {
        matches!(v.to_lowercase().as_str(), "darker" | "lighter")
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_variant_navigation_darker() {
        assert!(is_variant_navigation(&Some("darker".to_string())));
    }

    #[test]
    fn test_is_variant_navigation_lighter() {
        assert!(is_variant_navigation(&Some("lighter".to_string())));
    }

    #[test]
    fn test_is_variant_navigation_case_insensitive() {
        assert!(is_variant_navigation(&Some("DARKER".to_string())));
        assert!(is_variant_navigation(&Some("Lighter".to_string())));
    }

    #[test]
    fn test_is_variant_navigation_normal_variant() {
        assert!(!is_variant_navigation(&Some("dark".to_string())));
        assert!(!is_variant_navigation(&Some("light".to_string())));
    }

    #[test]
    fn test_is_variant_navigation_none() {
        assert!(!is_variant_navigation(&None));
    }

    // ── CLI parsing tests ──

    #[test]
    fn test_parse_theme_list() {
        let cli = Cli::try_parse_from(["vogix", "theme", "list"]).unwrap();
        assert!(matches!(
            cli.command,
            Commands::Theme {
                command: ThemeCommands::List { .. }
            }
        ));
    }

    #[test]
    fn test_parse_theme_status() {
        let cli = Cli::try_parse_from(["vogix", "theme", "status"]).unwrap();
        assert!(matches!(
            cli.command,
            Commands::Theme {
                command: ThemeCommands::Status
            }
        ));
    }

    #[test]
    fn test_parse_theme_set_with_flags() {
        let cli = Cli::try_parse_from(["vogix", "theme", "set", "-t", "catppuccin", "-v", "mocha"])
            .unwrap();
        if let Commands::Theme {
            command: ThemeCommands::Set { theme, variant, .. },
        } = cli.command
        {
            assert_eq!(theme.unwrap(), "catppuccin");
            assert_eq!(variant.unwrap(), "mocha");
        } else {
            panic!("Expected Theme Set command");
        }
    }

    #[test]
    fn test_parse_theme_refresh() {
        let cli = Cli::try_parse_from(["vogix", "theme", "refresh"]).unwrap();
        assert!(matches!(
            cli.command,
            Commands::Theme {
                command: ThemeCommands::Refresh { .. }
            }
        ));
    }

    #[test]
    fn test_parse_session_save_default() {
        let cli = Cli::try_parse_from(["vogix", "session", "save"]).unwrap();
        if let Commands::Session {
            command: SessionCommands::Save { name },
        } = cli.command
        {
            assert_eq!(name, "last");
        } else {
            panic!("Expected Session Save command");
        }
    }

    #[test]
    fn test_parse_session_save_named() {
        let cli = Cli::try_parse_from(["vogix", "session", "save", "work"]).unwrap();
        if let Commands::Session {
            command: SessionCommands::Save { name },
        } = cli.command
        {
            assert_eq!(name, "work");
        } else {
            panic!("Expected Session Save command");
        }
    }

    #[test]
    fn test_parse_session_restore() {
        let cli = Cli::try_parse_from(["vogix", "session", "restore"]).unwrap();
        assert!(matches!(
            cli.command,
            Commands::Session {
                command: SessionCommands::Restore { .. }
            }
        ));
    }

    #[test]
    fn test_parse_session_list() {
        let cli = Cli::try_parse_from(["vogix", "session", "list"]).unwrap();
        assert!(matches!(
            cli.command,
            Commands::Session {
                command: SessionCommands::List
            }
        ));
    }

    #[test]
    fn test_parse_daemon() {
        let cli = Cli::try_parse_from(["vogix", "daemon"]).unwrap();
        assert!(matches!(cli.command, Commands::Daemon));
    }

    #[test]
    fn test_parse_cache_clean() {
        let cli = Cli::try_parse_from(["vogix", "cache", "clean"]).unwrap();
        assert!(matches!(
            cli.command,
            Commands::Cache {
                command: CacheCommands::Clean
            }
        ));
    }

    // ── Shader command tests ──

    #[test]
    fn test_parse_shader_on() {
        let cli = Cli::try_parse_from(["vogix", "shader", "on"]).unwrap();
        assert!(matches!(
            cli.command,
            Commands::Shader {
                command: ShaderCommands::On
            }
        ));
    }

    #[test]
    fn test_parse_shader_off() {
        let cli = Cli::try_parse_from(["vogix", "shader", "off"]).unwrap();
        assert!(matches!(
            cli.command,
            Commands::Shader {
                command: ShaderCommands::Off
            }
        ));
    }

    #[test]
    fn test_parse_shader_toggle() {
        let cli = Cli::try_parse_from(["vogix", "shader", "toggle"]).unwrap();
        assert!(matches!(
            cli.command,
            Commands::Shader {
                command: ShaderCommands::Toggle
            }
        ));
    }

    // ── Property: all valid subcommands parse without error ──

    #[test]
    fn test_all_subcommands_parse() {
        let valid_commands = [
            vec!["vogix", "theme", "list"],
            vec!["vogix", "theme", "status"],
            vec!["vogix", "theme", "set", "-t", "test"],
            vec!["vogix", "theme", "set", "-v", "darker"],
            vec!["vogix", "theme", "set", "-s", "base16"],
            vec!["vogix", "theme", "refresh"],
            vec!["vogix", "session", "save"],
            vec!["vogix", "session", "save", "myname"],
            vec!["vogix", "session", "restore"],
            vec!["vogix", "session", "restore", "myname"],
            vec!["vogix", "session", "list"],
            vec!["vogix", "daemon"],
            vec!["vogix", "cache", "clean"],
            vec!["vogix", "shader", "on"],
            vec!["vogix", "shader", "off"],
            vec!["vogix", "shader", "toggle"],
        ];
        for args in &valid_commands {
            assert!(
                Cli::try_parse_from(args).is_ok(),
                "Failed to parse: {:?}",
                args
            );
        }
    }

    // ── Property: invalid subcommands fail gracefully ──

    #[test]
    fn test_invalid_commands_fail() {
        let invalid_commands = [
            vec!["vogix", "invalid"],
            vec!["vogix", "theme", "invalid"],
            vec!["vogix", "session", "invalid"],
            vec!["vogix", "theme", "set"], // set with no flags is valid but does nothing
        ];
        // "set" with no flags is actually valid (clap allows optional args)
        // Only truly invalid subcommands should fail
        assert!(Cli::try_parse_from(["vogix", "invalid"]).is_err());
        assert!(Cli::try_parse_from(["vogix", "theme", "invalid"]).is_err());
        assert!(Cli::try_parse_from(["vogix", "session", "invalid"]).is_err());
    }
}

mod cache;
mod cli;
mod commands;
mod config;
mod errors;
mod reload;
mod scheme;
mod shader;
mod state;
mod symlink;
mod template;
mod theme;

use cli::{CacheCommands, Cli, Commands, SessionCommands, ShaderCommands, ThemeCommands};
use commands::{
    handle_cache_clean, handle_completions, handle_daemon, handle_list, handle_refresh,
    handle_session_list, handle_session_restore, handle_session_restore_file, handle_session_save,
    handle_session_undo, handle_status, handle_theme_change,
};
use errors::Result;
use log::error;

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp(None)
        .format_target(false)
        .init();

    if let Err(e) = run() {
        error!("{}", e);
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse_args();

    match cli.command {
        Commands::Theme { command } => match command {
            ThemeCommands::List { scheme, variants } => handle_list(scheme.as_ref(), variants),
            ThemeCommands::Status => handle_status(),
            ThemeCommands::Set {
                scheme,
                theme,
                variant,
                quiet,
            } => handle_theme_change(scheme, theme, variant, quiet),
            ThemeCommands::Refresh { quiet } => handle_refresh(quiet),
        },

        Commands::Session { command } => match command {
            SessionCommands::Save { name } => handle_session_save(&name),
            SessionCommands::Restore {
                name,
                json,
                dry_run,
            } => {
                if let Some(path) = json {
                    handle_session_restore_file(&path, dry_run)
                } else {
                    handle_session_restore(&name, dry_run)
                }
            }
            SessionCommands::List => handle_session_list(),
            SessionCommands::Undo => handle_session_undo(),
        },

        Commands::Shader { command } => match command {
            ShaderCommands::On => commands::shader::handle_shader_on(),
            ShaderCommands::Off => commands::shader::handle_shader_off(),
            ShaderCommands::Toggle => commands::shader::handle_shader_toggle(),
        },

        Commands::Completions { shell } => handle_completions(shell),

        Commands::Cache { command } => match command {
            CacheCommands::Clean => handle_cache_clean(),
        },

        Commands::Daemon => handle_daemon(),
    }
}

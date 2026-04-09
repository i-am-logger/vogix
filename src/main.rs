mod cache;
mod cli;
mod commands;
mod config;
mod engine;
mod errors;
mod history;
mod reload;
mod scheme;
mod shader;
mod state;
mod symlink;
mod template;
mod theme;

use cli::{CacheCommands, Cli, Commands, SessionCommands, ShaderCommands, ThemeCommands};
use commands::{
    handle_cache_clean, handle_completions, handle_daemon, handle_list,
    handle_session_list, handle_session_restore, handle_session_restore_file, handle_session_save,
    handle_session_undo, handle_status,
};
use engine::{ShaderParam, VogixAction};
use errors::Result;
use log::{debug, error, info, warn};
use praxis::engine::{Action, Situation};

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

    match &cli.command {
        // ── Read-only commands — no engine needed ──
        Commands::Theme {
            command: ThemeCommands::List { scheme, variants },
        } => handle_list(scheme.as_ref(), *variants),

        Commands::Theme {
            command: ThemeCommands::Status,
        } => handle_status(),

        Commands::Session {
            command: SessionCommands::List,
        } => handle_session_list(),

        Commands::Session {
            command: SessionCommands::Save { name },
        } => handle_session_save(name),

        Commands::Session {
            command:
                SessionCommands::Restore {
                    name,
                    json,
                    dry_run,
                },
        } => {
            if let Some(path) = json {
                handle_session_restore_file(path, *dry_run)
            } else {
                handle_session_restore(name, *dry_run)
            }
        }

        Commands::Session {
            command: SessionCommands::Undo,
        } => handle_session_undo(),

        Commands::Shader {
            command: ShaderCommands::Status,
        } => commands::shader::handle_shader_status(),

        Commands::Completions { shell } => handle_completions(*shell),

        Commands::Cache {
            command: CacheCommands::Clean,
        } => handle_cache_clean(),

        Commands::Daemon => handle_daemon(),

        // ── Undo/redo — restore from history, not engine ──
        Commands::Theme {
            command: ThemeCommands::Undo,
        } => handle_theme_undo(),

        Commands::Theme {
            command: ThemeCommands::Redo,
        } => handle_theme_redo(),

        // ── State-mutating commands — go through praxis engine ──
        _ => run_with_engine(&cli.command),
    }
}

/// Route state-mutating commands through the praxis engine.
/// Flow: load state → resolve action → engine.next() → save history → save state → side effects
fn run_with_engine(command: &Commands) -> Result<()> {
    let state = state::State::load()?;
    let config = config::Config::load()?;

    // Translate CLI command to VogixAction (variant resolution happens here)
    let action = cli_to_action(command, &state)?;
    debug!("Action: {}", action.describe());

    // Run through engine (preconditions checked, pure state transition)
    let engine = engine::create_engine(state.clone());
    let engine = match engine.next(action) {
        Ok(e) => e,
        Err(err) => {
            let violations = match err {
                praxis::engine::EngineError::Violated { violations, .. } => violations,
                praxis::engine::EngineError::LogicalError { reason, .. } => {
                    return Err(errors::VogixError::Config(format!(
                        "Engine error: {}",
                        reason
                    )));
                }
            };
            for v in &violations {
                if !v.is_satisfied() {
                    error!("{}: {}", v.rule(), v.reason());
                }
            }
            return Ok(());
        }
    };

    let new_state = engine.situation();

    // Skip if no state change
    if *new_state == state {
        info!("No changes to apply");
        return Ok(());
    }

    // Verify theme-variant exists before committing state
    // (prevents persisting invalid state if theme was deleted)
    theme::verify_theme_variant_exists(&new_state.current_theme, &new_state.current_variant)?;

    // Side effects first — if they fail, state is NOT persisted
    execute_side_effects(command, &config, new_state)?;

    // Commit: push history and persist state only after side effects succeed
    let mut hist = history::History::load()?;
    hist.push(&state);
    hist.save()?;
    new_state.save()?;
    debug!("State saved: {}", new_state.describe());

    Ok(())
}

/// Undo last theme change — restore previous state from history
fn handle_theme_undo() -> Result<()> {
    let state = state::State::load()?;
    let config = config::Config::load()?;
    let mut hist = history::History::load()?;

    match hist.undo(&state) {
        Some(prev) => {
            prev.save()?;
            hist.save()?;
            info!("Undo → {}", prev.describe());

            // Re-apply everything for the restored state
            execute_side_effects(
                &Commands::Theme {
                    command: ThemeCommands::Refresh { quiet: false },
                },
                &config,
                &prev,
            )?;
            Ok(())
        }
        None => {
            info!("Nothing to undo");
            Ok(())
        }
    }
}

/// Redo last undone theme change
fn handle_theme_redo() -> Result<()> {
    let state = state::State::load()?;
    let config = config::Config::load()?;
    let mut hist = history::History::load()?;

    match hist.redo(&state) {
        Some(next) => {
            next.save()?;
            hist.save()?;
            info!("Redo → {}", next.describe());

            execute_side_effects(
                &Commands::Theme {
                    command: ThemeCommands::Refresh { quiet: false },
                },
                &config,
                &next,
            )?;
            Ok(())
        }
        None => {
            info!("Nothing to redo");
            Ok(())
        }
    }
}

/// Translate CLI command to VogixAction.
/// Variant resolution (filesystem I/O) happens here — before the pure engine.
fn cli_to_action(command: &Commands, state: &state::State) -> Result<VogixAction> {
    match command {
        Commands::Theme {
            command:
                ThemeCommands::Set {
                    scheme,
                    theme,
                    variant,
                    ..
                },
        } => {
            // Resolve variant: navigation (darker/lighter), polarity (dark/light), or exact name
            let resolved_variant = if let Some(v) = variant {
                if cli::is_variant_navigation(&Some(v.clone())) {
                    Some(commands::theme_change::navigate_variant(state, v)?)
                } else {
                    let theme_name = theme.as_deref().unwrap_or(&state.current_theme);
                    Some(commands::theme_change::resolve_variant(
                        theme_name,
                        v,
                        &state.current_variant,
                    )?)
                }
            } else if theme.is_some() && theme.as_deref() != Some(&state.current_theme) {
                // Theme changed, no variant specified — resolve polarity-matching variant
                commands::theme_change::resolve_polarity_variant(state, theme.as_deref().unwrap())?
            } else {
                None
            };

            Ok(VogixAction::SetTheme {
                scheme: *scheme,
                theme: theme.clone(),
                variant: resolved_variant,
            })
        }

        Commands::Theme {
            command: ThemeCommands::Refresh { .. },
        } => Ok(VogixAction::Refresh),

        Commands::Shader {
            command:
                ShaderCommands::On {
                    intensity,
                    brightness,
                    saturation,
                },
        } => Ok(VogixAction::ShaderOn {
            intensity: *intensity,
            brightness: *brightness,
            saturation: *saturation,
        }),

        Commands::Shader {
            command: ShaderCommands::Off,
        } => Ok(VogixAction::ShaderOff),

        Commands::Shader {
            command: ShaderCommands::Toggle,
        } => Ok(VogixAction::ShaderToggle),

        Commands::Shader {
            command: ShaderCommands::Intensity { value },
        } => Ok(VogixAction::ShaderParam {
            param: ShaderParam::Intensity,
            value: *value,
        }),

        Commands::Shader {
            command: ShaderCommands::Brightness { value },
        } => Ok(VogixAction::ShaderParam {
            param: ShaderParam::Brightness,
            value: *value,
        }),

        Commands::Shader {
            command: ShaderCommands::Saturation { value },
        } => Ok(VogixAction::ShaderParam {
            param: ShaderParam::Saturation,
            value: *value,
        }),

        _ => unreachable!("non-mutating commands handled before engine"),
    }
}

/// Execute side effects AFTER the engine has committed a state transition.
fn execute_side_effects(
    command: &Commands,
    config: &config::Config,
    state: &state::State,
) -> Result<()> {
    match command {
        Commands::Theme {
            command: ThemeCommands::Set { quiet, .. },
        }
        | Commands::Theme {
            command: ThemeCommands::Refresh { quiet },
        } => {
            // Template rendering
            if let Some(cache_path) = commands::refresh::maybe_render_templates(config, state)? {
                debug!(
                    "Using template-rendered configs from: {}",
                    cache_path.display()
                );
            }

            // Symlink update
            let symlink_manager = symlink::SymlinkManager::new();
            symlink_manager.update_current_symlink(&state.current_theme, &state.current_variant)?;

            // App reload
            let reload_dispatcher = reload::ReloadDispatcher::new();
            let reload_result = reload_dispatcher.reload_apps(config, *quiet);

            // Hardware color push
            if !config.hardware.is_empty()
                && let Some(theme_sources) = &config.theme_sources
            {
                    let variant_path = cache::paths::theme_variant_path(
                        theme_sources,
                        &state.current_scheme,
                        &state.current_theme,
                        &state.current_variant,
                    );
                    match theme::load_theme_colors(&variant_path, state.current_scheme) {
                        Ok(colors) => reload_dispatcher.apply_hardware(config, &colors, *quiet),
                        Err(e) => warn!("Hardware theme apply skipped: {}", e),
                    }
            }

            // Shader auto-apply
            if let Err(e) = commands::shader::maybe_apply_shader(config, state) {
                warn!("Shader apply failed: {}", e);
            }

            let theme_variant = format!("{}-{}", state.current_theme, state.current_variant);
            if reload_result.has_failures() {
                warn!(
                    "Applied: {} ({}/{} reloaded, {} failed)",
                    theme_variant,
                    reload_result.success_count,
                    reload_result.total_count,
                    reload_result.failed_apps.len()
                );
            } else {
                info!("Applied: {}", theme_variant);
            }
        }

        Commands::Shader { .. } => {
            // Apply or clear shader based on new state
            if let Err(e) = commands::shader::maybe_apply_shader(config, state) {
                warn!("Shader apply failed: {}", e);
            }
        }

        _ => {}
    }

    Ok(())
}

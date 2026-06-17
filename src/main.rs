mod cache;
mod cli;
mod commands;
mod config;
mod engine;
mod errors;
mod history;
mod input;
mod reload;
mod scheme;
mod shader;
mod state;
mod symlink;
mod template;
mod theme;

use cli::{
    CacheCommands, Cli, Commands, InputCommands, LogLevel, ModesCommands, SessionCommands,
    ShaderCommands, ThemeCommands,
};
use commands::{
    handle_cache_clean, handle_completions, handle_daemon, handle_input_check, handle_input_doctor,
    handle_input_help, handle_input_run, handle_list, handle_modes_confusion, handle_modes_recent,
    handle_modes_stats, handle_session_list, handle_session_restore, handle_session_restore_file,
    handle_session_save, handle_session_undo, handle_status,
};
use engine::{ShaderParam, VogixAction};
use errors::Result;
use log::{debug, error, info, warn};

fn main() {
    // Restore default SIGPIPE handling. Rust sets SIGPIPE to SIG_IGN at startup,
    // so writing to a pipe whose reader has already closed (e.g.
    // `vogix completions bash | head`, `vogix theme list | grep -q`) surfaces as a
    // BrokenPipe io error that clap_complete/println `.unwrap()` turns into a
    // panic. SIG_DFL makes the process exit quietly on SIGPIPE — the conventional
    // CLI behavior.
    // SAFETY: called once at the very start of main, before any threads spawn.
    #[cfg(unix)]
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    }

    let cli = Cli::parse_args();
    init_logging(cli.log_level);

    if let Err(e) = run(&cli) {
        error!("{}", e);
        std::process::exit(1);
    }
}

/// Initialise `env_logger`. A `--log-level` flag (if given) overrides `RUST_LOG`;
/// otherwise `RUST_LOG` — or the built-in `info` default — applies. Either way
/// the level flows to stderr/journald, so `journalctl --user -u vogix-input`
/// shows whatever the unit's `RUST_LOG=vogix=<level>` (or this flag) selects.
fn init_logging(level: Option<LogLevel>) {
    let mut builder =
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"));
    if let Some(level) = level {
        builder.parse_filters(level.as_filter());
    }
    builder.format_timestamp(None).format_target(false).init();
}

fn run(cli: &Cli) -> Result<()> {
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

        Commands::Input {
            command: InputCommands::Check { config },
        } => handle_input_check(config.as_deref()),

        Commands::Input {
            command: InputCommands::Run { config },
        } => handle_input_run(config.as_deref()),

        Commands::Input {
            command: InputCommands::Doctor { watch },
        } => handle_input_doctor(*watch),

        Commands::Input {
            command: InputCommands::Keys { print, config },
        } => handle_input_help(*print, config.as_deref()),

        Commands::Modes {
            command: ModesCommands::Recent { count },
        } => handle_modes_recent(*count),

        Commands::Modes {
            command: ModesCommands::Stats,
        } => handle_modes_stats(),

        Commands::Modes {
            command: ModesCommands::Confusion { threshold_ms },
        } => handle_modes_confusion(*threshold_ms),

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
    let action = cli_to_action(command, &state, &config)?;
    debug!("Action: {}", action.describe());

    // Run through engine (preconditions checked, pure state transition)
    let engine = engine::create_engine(state.clone());
    let engine = match engine.next(action) {
        Ok(e) => e,
        Err(err) => {
            let violations = match err {
                pr4xis::engine::EngineError::Violated { violations, .. } => violations,
                pr4xis::engine::EngineError::LogicalError { counterexample, .. } => {
                    return Err(errors::VogixError::Config(format!(
                        "Engine error: {}",
                        counterexample.meta().description.as_str()
                    )));
                }
            };
            for v in &violations {
                let m = v.meta();
                error!("{}: {}", m.name.as_str(), m.description.as_str());
            }
            return Ok(());
        }
    };

    let new_state = engine.situation();

    // Refresh is intentionally a no-op state transition — its whole purpose is
    // to re-run side effects (templates, symlinks, app reloads, hardware, shader).
    // For all other actions, skip when nothing actually changed.
    let is_refresh = matches!(
        command,
        Commands::Theme {
            command: ThemeCommands::Refresh { .. },
        },
    );
    if !is_refresh && *new_state == state {
        info!("No changes to apply");
        return Ok(());
    }

    // Verify theme-variant exists before committing state (prevents persisting
    // invalid state if the theme was deleted) — but only when the theme/variant
    // actually changed, or on Refresh. Shader/mode-only actions never touch the
    // theme directory, so they must not abort just because a stale theme package
    // referenced by on-disk state was removed.
    if is_refresh
        || new_state.current_theme != state.current_theme
        || new_state.current_variant != state.current_variant
    {
        theme::verify_theme_variant_exists(&new_state.current_theme, &new_state.current_variant)?;
    }

    // Side effects first — if they fail, state is NOT persisted
    execute_side_effects(command, &config, new_state)?;

    // Refresh deliberately produces no state change; don't pollute history.
    if *new_state == state {
        return Ok(());
    }

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
        Some(mut prev) => {
            // History snapshots carry the current_mode captured when they were
            // pushed, but the daemon owns current_mode and updates it out-of-band
            // (via save_current_mode, no history push). Carry the live mode forward
            // so a theme undo doesn't revert the interaction mode to a stale value.
            prev.current_mode = state.current_mode.clone();
            // Side effects first — if they fail, state is NOT persisted (same as engine flow)
            execute_side_effects(
                &Commands::Theme {
                    command: ThemeCommands::Refresh { quiet: false },
                },
                &config,
                &prev,
            )?;
            prev.save()?;
            hist.save()?;
            info!("Undo → {}", prev.describe());
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
        Some(mut next) => {
            // Carry the live (daemon-owned) current_mode forward — see handle_theme_undo.
            next.current_mode = state.current_mode.clone();
            // Side effects first — if they fail, state is NOT persisted (same as engine flow)
            execute_side_effects(
                &Commands::Theme {
                    command: ThemeCommands::Refresh { quiet: false },
                },
                &config,
                &next,
            )?;
            next.save()?;
            hist.save()?;
            info!("Redo → {}", next.describe());
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
fn cli_to_action(
    command: &Commands,
    state: &state::State,
    config: &config::Config,
) -> Result<VogixAction> {
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
            // Fall back to the user's configured shader defaults (not hardcoded
            // 0.5/1.0/1.0) when a flag is omitted; the engine only reaches its own
            // constants when config.shader is absent entirely.
            intensity: (*intensity).or(config.shader.as_ref().map(|c| c.intensity)),
            brightness: (*brightness).or(config.shader.as_ref().map(|c| c.brightness)),
            saturation: (*saturation).or(config.shader.as_ref().map(|c| c.saturation)),
        }),

        Commands::Shader {
            command: ShaderCommands::Off,
        } => Ok(VogixAction::ShaderOff),

        Commands::Shader {
            command: ShaderCommands::Toggle,
        } => {
            // Seed the off→on direction from configured shader defaults so a
            // toggle honors config.shader instead of the hardcoded constants.
            let (intensity, brightness, saturation) = config
                .shader
                .as_ref()
                .map(|c| (c.intensity, c.brightness, c.saturation))
                .unwrap_or((0.5, 1.0, 1.0));
            Ok(VogixAction::ShaderToggle {
                intensity,
                brightness,
                saturation,
            })
        }

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

        Commands::Mode { target } => Ok(VogixAction::ModeChange {
            target: target.clone(),
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

            // Load theme colors for validation and hardware
            if let Some(theme_sources) = &config.theme_sources {
                let variant_path = cache::paths::theme_variant_path(
                    theme_sources,
                    &state.current_scheme,
                    &state.current_theme,
                    &state.current_variant,
                );
                match theme::load_theme_colors(&variant_path, state.current_scheme) {
                    Ok(colors) => {
                        // Validate palette against praxis axioms
                        let palette = theme::palette::build_palette(&colors, state.current_scheme);
                        let expected = state.current_scheme.slot_count();
                        if palette.len() < expected {
                            warn!(
                                "Theme palette has {} slots, expected {} for {}",
                                palette.len(),
                                expected,
                                state.current_scheme
                            );
                        }
                        if let Some(pol) = theme::palette::polarity(&palette) {
                            debug!("Theme polarity: {:?}", pol);
                        }
                        for failure in theme::palette::validate(&palette) {
                            warn!("Theme axiom: {}", failure);
                        }

                        // Hardware color push
                        if !config.hardware.is_empty() {
                            reload_dispatcher.apply_hardware(config, &colors, *quiet);
                        }
                    }
                    Err(e) => warn!("Theme colors not loaded: {}", e),
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

        Commands::Mode { target } => {
            // Dispatch Hyprland submap change — "app" maps to "reset" (default submap)
            let submap = if target == "app" { "reset" } else { target };
            match std::process::Command::new("hyprctl")
                .args(["dispatch", "submap", submap])
                .output()
            {
                Ok(output) if output.status.success() => {
                    info!("Mode: {} → {}", state.current_mode, target);
                }
                Ok(output) => {
                    warn!(
                        "Mode switch failed: {}",
                        String::from_utf8_lossy(&output.stderr).trim()
                    );
                }
                Err(e) => warn!("hyprctl not available: {}", e),
            }
        }

        _ => {}
    }

    Ok(())
}

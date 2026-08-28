//! `vogix hypr` — compositor IPC in whichever dialect the compositor runs.
//!
//! Keybindings and scripts used to shell out to `hyprctl dispatch …` /
//! `hyprctl --batch "keyword …"`, which hardcodes the legacy hyprlang
//! dialect; under the Lua engine those exact strings are rejected. These
//! verbs route the same intents through the provider-aware socket handle
//! ([`crate::input::hypr::Hypr`]), so a bound shell command works unchanged
//! across the hyprlang→Lua migration.

use crate::cli::HyprCommands;
use crate::errors::{Result, VogixError};
use crate::input::hypr::Hypr;

pub fn handle_hypr(command: &HyprCommands) -> Result<()> {
    let hypr = Hypr::discover().ok_or(VogixError::HyprlandNotRunning)?;
    match command {
        HyprCommands::Dispatch { action } => hypr.dispatch(action).map_err(to_err),
        HyprCommands::Keyword { args } => {
            if !args.len().is_multiple_of(2) {
                return Err(VogixError::Config(
                    "vogix hypr keyword takes alternating KEY VALUE arguments".to_string(),
                ));
            }
            let pairs: Vec<(&str, &str)> = args
                .chunks_exact(2)
                .map(|kv| (kv[0].as_str(), kv[1].as_str()))
                .collect();
            hypr.set_keywords(&pairs).map_err(to_err)
        }
    }
}

fn to_err(e: std::io::Error) -> VogixError {
    VogixError::HyprctlFailed {
        code: -1,
        detail: e.to_string(),
    }
}

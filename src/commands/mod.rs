//! Command handlers for vogix CLI.
//!
//! This module provides handlers for each CLI command:
//! - `list` - Show available themes and schemes
//! - `status` - Display current theme state
//! - `refresh` - Reapply current theme without changes
//! - `cache` - Manage template cache
//! - `completions` - Generate shell completions
//! - `theme_change` - Handle -t, -v, -s flags

mod cache;
mod completions;
mod daemon;
mod input;
mod list;
mod modes;
pub mod refresh;
pub mod session;
pub mod shader;
mod status;
pub mod theme_change;

pub use cache::handle_cache_clean;
pub use completions::handle_completions;
pub use daemon::handle_daemon;
pub use input::{handle_input_check, handle_input_doctor, handle_input_run};
pub use list::handle_list;
pub use modes::{handle_modes_confusion, handle_modes_recent, handle_modes_stats};
pub use session::{
    handle_session_list, handle_session_restore, handle_session_restore_file, handle_session_save,
    handle_session_undo,
};
pub use status::handle_status;

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
mod list;
mod refresh;
pub mod session;
mod status;
mod theme_change;

pub use cache::handle_cache_clean;
pub use completions::handle_completions;
pub use daemon::handle_daemon;
pub use list::handle_list;
pub use refresh::handle_refresh;
pub use session::{handle_session_list, handle_session_restore, handle_session_restore_file, handle_session_save, handle_session_undo};
pub use status::handle_status;
pub use theme_change::handle_theme_change;

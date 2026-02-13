//! TUI (Terminal User Interface) module for live feedback display.
//!
//! Provides a rich, animated dashboard during loop execution showing:
//! - Live AI output stream
//! - Real-time statistics
//! - Animated spinners and progress
//! - Task and iteration info

mod app;
/// Team TUI application state and event handling.
pub mod team_app;
/// Team TUI rendering.
pub mod team_ui;
mod ui;

pub use app::{TuiApp, TuiEvent};
pub use team_app::{TeamCommand, TeamTuiApp};

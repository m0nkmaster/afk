//! TUI (Terminal User Interface) module for live feedback display.
//!
//! Provides animated dashboards for both single-agent and team modes:
//! - **Single-agent TUI** (`TuiApp`): live output stream, stats, progress
//! - **Team TUI** (`TeamTuiApp`): multi-agent dashboard with overview and focus modes

mod app;
/// Team TUI application state and event handling.
pub mod team_app;
/// Team TUI rendering.
pub mod team_ui;
mod ui;

pub use app::{TuiApp, TuiEvent};
pub use team_app::{TeamCommand, TeamTuiApp};

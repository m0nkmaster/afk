//! TUI (Terminal User Interface) module for live feedback display.
//!
//! Provides a rich, animated dashboard during loop execution showing:
//! - Live AI output stream
//! - Real-time statistics
//! - Animated spinners and progress
//! - Task and iteration info

mod app;
mod ui;

pub use app::{TuiApp, TuiEvent};

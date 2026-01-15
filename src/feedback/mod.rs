//! Real-time feedback display.
//!
//! This module provides Rich-like live panels for displaying progress,
//! and metrics collection for iteration statistics.
//!
//! # Submodules
//!
//! - [`spinner`] - Inline spinners for showing progress during operations
//! - [`display`] - Panel-based displays for iteration feedback
//! - [`metrics`] - Metrics collection for iteration statistics
//! - [`art`] - ASCII art assets (mascots, spinners, fireworks)

mod art;
mod display;
mod metrics;
mod spinner;

pub use art::{
    get_burst_pattern, get_firework_char, get_mascot, get_spinner_frame, get_star_char,
    FIREWORK_BURSTS, FIREWORK_CHARS, MASCOT_STATES, SPINNERS, STAR_CHARS,
};
pub use display::{DisplayMode, FeedbackDisplay};
pub use metrics::{ActivityState, IterationMetrics, MetricsCollector};
pub use spinner::Spinner;

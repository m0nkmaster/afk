//! Real-time feedback display.
//!
//! This module provides Rich-like live panels for displaying progress,
//! and metrics collection for iteration statistics.

mod art;
mod display;
mod metrics;

pub use art::{
    get_burst_pattern, get_firework_char, get_mascot, get_spinner_frame, get_star_char,
    FIREWORK_BURSTS, FIREWORK_CHARS, MASCOT_STATES, SPINNERS, STAR_CHARS,
};
pub use display::{DisplayMode, FeedbackDisplay, Spinner};
pub use metrics::{ActivityState, IterationMetrics, MetricsCollector};

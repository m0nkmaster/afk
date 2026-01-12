//! Real-time feedback display.
//!
//! This module provides Rich-like live panels for displaying progress,
//! and metrics collection for iteration statistics.

mod art;
mod display;
mod metrics;

pub use art::{get_mascot, get_spinner_frame, MASCOT_STATES, SPINNERS};
pub use display::{DisplayMode, FeedbackDisplay};
pub use metrics::{ActivityState, IterationMetrics, MetricsCollector};

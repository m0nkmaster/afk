//! Real-time feedback display.
//!
//! This module provides Rich-like live panels for displaying progress,
//! and metrics collection for iteration statistics.

mod metrics;

pub use metrics::{ActivityState, IterationMetrics, MetricsCollector};

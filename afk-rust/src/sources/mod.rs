//! Task source adapters.
//!
//! This module aggregates tasks from various sources (beads, json, markdown, github).

pub mod beads;
pub mod json;
pub mod markdown;

pub use beads::{close_beads_issue, load_beads_tasks};
pub use json::load_json_tasks;
pub use markdown::load_markdown_tasks;

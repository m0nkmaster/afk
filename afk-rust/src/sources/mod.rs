//! Task source adapters.
//!
//! This module aggregates tasks from various sources (beads, json, markdown, github).

pub mod json;
pub mod markdown;

pub use json::load_json_tasks;
pub use markdown::load_markdown_tasks;

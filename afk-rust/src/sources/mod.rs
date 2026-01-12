//! Task source adapters.
//!
//! This module aggregates tasks from various sources (beads, json, markdown, github).

pub mod json;

pub use json::load_json_tasks;

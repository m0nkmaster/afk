//! Prompt generation with templates.
//!
//! This module generates prompts for AI CLI tools using Tera templates.

pub mod template;

// Re-export key types and functions for convenience.
pub use template::{get_template, get_template_with_root, DEFAULT_TEMPLATE};

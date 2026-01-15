//! Shared utility for matching paths against ignore patterns.
//!
//! This module provides a reusable `PathMatcher` for checking if paths
//! should be ignored based on glob-like patterns.

use std::path::Path;

/// Default patterns for common directories and files that should be ignored.
///
/// Used by file watchers and other components that need to skip build artefacts,
/// version control directories, and dependency folders.
pub const DEFAULT_IGNORE_PATTERNS: &[&str] = &[
    ".git",
    "__pycache__",
    "node_modules",
    ".venv",
    ".afk",
    "target",
    ".mypy_cache",
    ".pytest_cache",
    ".ruff_cache",
    "*.pyc",
    "*.pyo",
];

/// A matcher for checking paths against ignore patterns.
///
/// Supports two pattern types:
/// - `*.ext` - matches files ending with `.ext` (extension matching)
/// - `name` - matches paths containing `name` anywhere (substring matching)
///
/// # Examples
///
/// ```
/// use afk::path_matcher::PathMatcher;
/// use std::path::Path;
///
/// let matcher = PathMatcher::new(&[".git", "*.pyc"]);
///
/// assert!(matcher.matches(Path::new("/project/.git/config")));
/// assert!(matcher.matches(Path::new("/project/cache/file.pyc")));
/// assert!(!matcher.matches(Path::new("/project/src/main.rs")));
/// ```
#[derive(Debug, Clone)]
pub struct PathMatcher {
    patterns: Vec<String>,
}

impl PathMatcher {
    /// Create a new PathMatcher from a slice of pattern strings.
    pub fn new(patterns: &[&str]) -> Self {
        Self {
            patterns: patterns.iter().map(|s| (*s).to_string()).collect(),
        }
    }

    /// Create a new PathMatcher from owned String patterns.
    pub fn from_strings(patterns: Vec<String>) -> Self {
        Self { patterns }
    }

    /// Create a PathMatcher with the default ignore patterns.
    pub fn with_defaults() -> Self {
        Self::new(DEFAULT_IGNORE_PATTERNS)
    }

    /// Add a pattern to the matcher.
    pub fn add_pattern(&mut self, pattern: &str) {
        self.patterns.push(pattern.to_string());
    }

    /// Check if a path matches any of the ignore patterns.
    ///
    /// Returns `true` if the path should be ignored.
    pub fn matches(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy();
        self.patterns.iter().any(|pattern| {
            if let Some(suffix) = pattern.strip_prefix('*') {
                // Extension pattern like *.pyc
                path_str.ends_with(suffix)
            } else {
                // Directory/path component substring match
                path_str.contains(pattern)
            }
        })
    }

    /// Get the current patterns.
    pub fn patterns(&self) -> &[String] {
        &self.patterns
    }
}

impl Default for PathMatcher {
    fn default() -> Self {
        Self::with_defaults()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_matcher() {
        let matcher = PathMatcher::new(&[".git", "*.pyc"]);
        assert_eq!(matcher.patterns().len(), 2);
    }

    #[test]
    fn test_from_strings() {
        let patterns = vec!["custom".to_string(), "pattern".to_string()];
        let matcher = PathMatcher::from_strings(patterns);
        assert_eq!(matcher.patterns().len(), 2);
    }

    #[test]
    fn test_with_defaults() {
        let matcher = PathMatcher::with_defaults();
        assert!(matcher.patterns().contains(&".git".to_string()));
        assert!(matcher.patterns().contains(&"node_modules".to_string()));
        assert!(matcher.patterns().contains(&".afk".to_string()));
    }

    #[test]
    fn test_add_pattern() {
        let mut matcher = PathMatcher::new(&[]);
        matcher.add_pattern("new_pattern");
        assert!(matcher.patterns().contains(&"new_pattern".to_string()));
    }

    #[test]
    fn test_matches_git() {
        let matcher = PathMatcher::with_defaults();
        assert!(matcher.matches(Path::new("/project/.git/config")));
        assert!(matcher.matches(Path::new("/project/.git/objects/abc")));
    }

    #[test]
    fn test_matches_node_modules() {
        let matcher = PathMatcher::with_defaults();
        assert!(matcher.matches(Path::new("/project/node_modules/pkg/file.js")));
    }

    #[test]
    fn test_matches_extension() {
        let matcher = PathMatcher::with_defaults();
        assert!(matcher.matches(Path::new("/project/file.pyc")));
        assert!(matcher.matches(Path::new("/project/cache/module.pyo")));
    }

    #[test]
    fn test_does_not_match_normal_files() {
        let matcher = PathMatcher::with_defaults();
        assert!(!matcher.matches(Path::new("/project/src/main.rs")));
        assert!(!matcher.matches(Path::new("/project/lib/utils.py")));
    }

    #[test]
    fn test_empty_matcher() {
        let matcher = PathMatcher::new(&[]);
        assert!(!matcher.matches(Path::new("/anything/at/all")));
    }

    #[test]
    fn test_default_trait() {
        let matcher = PathMatcher::default();
        assert!(matcher.patterns().contains(&".git".to_string()));
    }

    #[test]
    fn test_clone() {
        let matcher = PathMatcher::new(&[".git"]);
        let cloned = matcher.clone();
        assert_eq!(cloned.patterns(), matcher.patterns());
    }
}

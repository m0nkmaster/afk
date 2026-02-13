//! Persona management for team mode.
//!
//! Personas are markdown files in `.afk/personas/` that define the perspective
//! and instruction set for each agent in team mode. Each file has YAML
//! frontmatter (name, emoji) and a markdown body (the instruction).

use std::fs;
use std::path::{Path, PathBuf};

/// Default directory for persona files.
pub const PERSONAS_DIR: &str = ".afk/personas";

/// A persona defines an agent's perspective and behaviour.
#[derive(Debug, Clone)]
pub struct Persona {
    /// Display name (e.g., "Builder").
    pub name: String,
    /// Emoji for TUI display (e.g., "🔧").
    pub emoji: String,
    /// Instruction text prepended to the agent's prompt.
    pub instruction: String,
    /// Source file path.
    pub path: PathBuf,
}

/// Error type for persona operations.
#[derive(Debug, thiserror::Error)]
pub enum PersonaError {
    /// Error reading persona files from disk.
    #[error("Failed to read persona directory: {0}")]
    ReadError(#[from] std::io::Error),
    /// Error parsing persona file frontmatter.
    #[error("Failed to parse persona file {path}: {reason}")]
    ParseError {
        /// Path to the file that failed to parse.
        path: String,
        /// Reason for the parse failure.
        reason: String,
    },
}

/// Default persona definitions (written on first `afk team` run).
const DEFAULT_PERSONAS: &[(&str, &str, &str, &str)] = &[
    (
        "builder.md",
        "Builder",
        "🔧",
        "Focus on clean implementation. Get it working, keep it simple.\n\
         Follow existing code patterns and conventions in the project.\n\
         Prefer standard library solutions over adding new dependencies.\n\
         When in doubt, choose the straightforward approach.",
    ),
    (
        "critic.md",
        "Critic",
        "🔍",
        "Be thorough and careful. Think about what could go wrong.\n\
         Add proper error handling for edge cases.\n\
         Validate inputs and handle failure gracefully.\n\
         If you see existing code that's fragile, strengthen it while you're there.",
    ),
    (
        "tester.md",
        "Tester",
        "🧪",
        "Write tests first where practical. Think about edge cases.\n\
         Verify that existing tests still pass after your changes.\n\
         Add both happy-path and error-path test cases.\n\
         If test infrastructure is missing, set it up.",
    ),
];

/// Load all personas from the personas directory.
///
/// If the directory doesn't exist or is empty, returns an empty vec.
/// Persona files are `.md` files with YAML frontmatter.
pub fn load_personas(root: Option<&Path>) -> Result<Vec<Persona>, PersonaError> {
    let dir = personas_dir(root);

    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut personas = Vec::new();
    let mut entries: Vec<_> = fs::read_dir(&dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|ext| ext == "md").unwrap_or(false))
        .collect();

    // Sort by filename for deterministic ordering
    entries.sort_by_key(|e| e.file_name());

    for entry in entries {
        let path = entry.path();
        let content = fs::read_to_string(&path)?;
        let persona = parse_persona(&content, &path)?;
        personas.push(persona);
    }

    Ok(personas)
}

/// Ensure default persona files exist.
///
/// Creates `.afk/personas/` and writes default persona files if the
/// directory doesn't exist or is empty.
///
/// Returns the number of persona files created.
pub fn ensure_defaults(root: Option<&Path>) -> Result<u32, PersonaError> {
    let dir = personas_dir(root);

    // Check if directory exists and has .md files
    let has_personas = dir.exists()
        && fs::read_dir(&dir)
            .map(|entries| {
                entries
                    .filter_map(|e| e.ok())
                    .any(|e| e.path().extension().map(|ext| ext == "md").unwrap_or(false))
            })
            .unwrap_or(false);

    if has_personas {
        return Ok(0);
    }

    fs::create_dir_all(&dir)?;

    let mut created = 0;
    for (filename, name, emoji, instruction) in DEFAULT_PERSONAS {
        let path = dir.join(filename);
        if !path.exists() {
            let content = format!("---\nname: {name}\nemoji: {emoji}\n---\n\n{instruction}\n");
            fs::write(&path, content)?;
            created += 1;
        }
    }

    Ok(created)
}

/// Parse a persona from file content.
fn parse_persona(content: &str, path: &Path) -> Result<Persona, PersonaError> {
    let path_str = path.display().to_string();

    // Split frontmatter and body
    let (frontmatter, body) =
        split_frontmatter(content).ok_or_else(|| PersonaError::ParseError {
            path: path_str.clone(),
            reason: "Missing YAML frontmatter (expected --- delimiters)".to_string(),
        })?;

    // Parse frontmatter fields
    let name = extract_field(&frontmatter, "name").unwrap_or_else(|| {
        // Fall back to filename without extension
        path.file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "Agent".to_string())
    });

    let emoji = extract_field(&frontmatter, "emoji").unwrap_or_else(|| "🤖".to_string());

    let instruction = body.trim().to_string();

    Ok(Persona {
        name,
        emoji,
        instruction,
        path: path.to_path_buf(),
    })
}

/// Split content into frontmatter and body.
///
/// Expects `---\n...\n---\n` at the start of the file.
fn split_frontmatter(content: &str) -> Option<(String, String)> {
    let content = content.trim_start();

    if !content.starts_with("---") {
        // No frontmatter — treat the whole thing as instruction with defaults
        return Some((String::new(), content.to_string()));
    }

    // Find the closing ---
    let after_first = &content[3..];
    let close_pos = after_first.find("\n---")?;
    let frontmatter = after_first[..close_pos].trim().to_string();
    let body = after_first[close_pos + 4..].to_string();

    Some((frontmatter, body))
}

/// Extract a simple `key: value` field from YAML-like frontmatter.
fn extract_field(frontmatter: &str, key: &str) -> Option<String> {
    let prefix = format!("{key}:");
    for line in frontmatter.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix(&prefix) {
            let value = rest.trim().to_string();
            if !value.is_empty() {
                return Some(value);
            }
        }
    }
    None
}

/// Get the personas directory path.
fn personas_dir(root: Option<&Path>) -> PathBuf {
    match root {
        Some(r) => r.join(PERSONAS_DIR),
        None => PathBuf::from(PERSONAS_DIR),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_parse_persona_full() {
        let content = "---\nname: Builder\nemoji: 🔧\n---\n\nFocus on clean code.\n";
        let path = Path::new("builder.md");
        let persona = parse_persona(content, path).unwrap();
        assert_eq!(persona.name, "Builder");
        assert_eq!(persona.emoji, "🔧");
        assert_eq!(persona.instruction, "Focus on clean code.");
    }

    #[test]
    fn test_parse_persona_no_frontmatter() {
        let content = "Just an instruction with no frontmatter.";
        let path = Path::new("agent.md");
        let persona = parse_persona(content, path).unwrap();
        assert_eq!(persona.name, "agent"); // Falls back to filename
        assert_eq!(persona.emoji, "🤖"); // Default emoji
        assert_eq!(
            persona.instruction,
            "Just an instruction with no frontmatter."
        );
    }

    #[test]
    fn test_parse_persona_missing_fields() {
        let content = "---\nname: OnlyName\n---\n\nDo stuff.\n";
        let path = Path::new("test.md");
        let persona = parse_persona(content, path).unwrap();
        assert_eq!(persona.name, "OnlyName");
        assert_eq!(persona.emoji, "🤖"); // Default
        assert_eq!(persona.instruction, "Do stuff.");
    }

    #[test]
    fn test_split_frontmatter() {
        let content = "---\nname: Test\n---\n\nBody here.";
        let (fm, body) = split_frontmatter(content).unwrap();
        assert_eq!(fm, "name: Test");
        assert!(body.contains("Body here."));
    }

    #[test]
    fn test_split_frontmatter_no_delimiters() {
        let content = "Just a body with no frontmatter.";
        let (fm, body) = split_frontmatter(content).unwrap();
        assert!(fm.is_empty());
        assert_eq!(body, "Just a body with no frontmatter.");
    }

    #[test]
    fn test_extract_field() {
        let fm = "name: Builder\nemoji: 🔧\nextra: stuff";
        assert_eq!(extract_field(fm, "name"), Some("Builder".to_string()));
        assert_eq!(extract_field(fm, "emoji"), Some("🔧".to_string()));
        assert_eq!(extract_field(fm, "extra"), Some("stuff".to_string()));
        assert_eq!(extract_field(fm, "missing"), None);
    }

    #[test]
    fn test_extract_field_empty_value() {
        let fm = "name:\nemoji: 🔧";
        assert_eq!(extract_field(fm, "name"), None);
        assert_eq!(extract_field(fm, "emoji"), Some("🔧".to_string()));
    }

    #[test]
    fn test_ensure_defaults_creates_files() {
        let temp = TempDir::new().unwrap();
        let created = ensure_defaults(Some(temp.path())).unwrap();
        assert_eq!(created, 3);

        let dir = temp.path().join(PERSONAS_DIR);
        assert!(dir.join("builder.md").exists());
        assert!(dir.join("critic.md").exists());
        assert!(dir.join("tester.md").exists());
    }

    #[test]
    fn test_ensure_defaults_idempotent() {
        let temp = TempDir::new().unwrap();
        ensure_defaults(Some(temp.path())).unwrap();
        let created = ensure_defaults(Some(temp.path())).unwrap();
        assert_eq!(created, 0); // Already exist
    }

    #[test]
    fn test_load_personas_empty() {
        let temp = TempDir::new().unwrap();
        let personas = load_personas(Some(temp.path())).unwrap();
        assert!(personas.is_empty());
    }

    #[test]
    fn test_load_personas_with_defaults() {
        let temp = TempDir::new().unwrap();
        ensure_defaults(Some(temp.path())).unwrap();

        let personas = load_personas(Some(temp.path())).unwrap();
        assert_eq!(personas.len(), 3);
        assert_eq!(personas[0].name, "Builder");
        assert_eq!(personas[1].name, "Critic");
        assert_eq!(personas[2].name, "Tester");
    }

    #[test]
    fn test_load_personas_custom() {
        let temp = TempDir::new().unwrap();
        let dir = temp.path().join(PERSONAS_DIR);
        fs::create_dir_all(&dir).unwrap();

        fs::write(
            dir.join("custom.md"),
            "---\nname: Custom Agent\nemoji: 🎯\n---\n\nDo custom things.\n",
        )
        .unwrap();

        let personas = load_personas(Some(temp.path())).unwrap();
        assert_eq!(personas.len(), 1);
        assert_eq!(personas[0].name, "Custom Agent");
        assert_eq!(personas[0].emoji, "🎯");
    }

    #[test]
    fn test_load_personas_ignores_non_md() {
        let temp = TempDir::new().unwrap();
        let dir = temp.path().join(PERSONAS_DIR);
        fs::create_dir_all(&dir).unwrap();

        fs::write(dir.join("notes.txt"), "not a persona").unwrap();
        fs::write(
            dir.join("real.md"),
            "---\nname: Real\nemoji: ✅\n---\n\nI am real.\n",
        )
        .unwrap();

        let personas = load_personas(Some(temp.path())).unwrap();
        assert_eq!(personas.len(), 1);
        assert_eq!(personas[0].name, "Real");
    }

    #[test]
    fn test_default_personas_parse_correctly() {
        // Verify all default persona content parses without error
        for (filename, name, emoji, instruction) in DEFAULT_PERSONAS {
            let content = format!("---\nname: {name}\nemoji: {emoji}\n---\n\n{instruction}\n");
            let path = Path::new(filename);
            let persona = parse_persona(&content, path).unwrap();
            assert_eq!(persona.name, *name);
            assert_eq!(persona.emoji, *emoji);
            assert!(!persona.instruction.is_empty());
        }
    }
}

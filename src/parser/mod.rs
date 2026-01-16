//! AI CLI output parsing.
//!
//! This module parses output from Claude, Cursor, and Aider CLI tools
//! to detect tool calls, file changes, errors, and warnings.
//!
//! ## Parsing Modes
//!
//! - **Regex-based**: For plain text output (legacy/fallback)
//! - **NDJSON stream-json**: For structured streaming output from Cursor/Claude CLIs

mod stream_json;

pub use stream_json::{CliFormat, StreamEvent, StreamJsonParser, ToolType};

use regex::Regex;
use std::sync::LazyLock;

/// Types of events that can be detected in AI CLI output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventType {
    /// A tool was invoked by the AI (e.g., write_file, execute_command).
    ToolCall,
    /// A file was modified, created, or deleted.
    FileChange,
    /// An error occurred during processing.
    Error,
    /// A warning was emitted during processing.
    Warning,
}

/// Base event trait for all parsed output events.
#[derive(Debug, Clone)]
pub struct Event {
    /// Type of the event.
    pub event_type: EventType,
    /// The raw line that triggered this event.
    pub raw_line: String,
}

/// Event for tool calls.
#[derive(Debug, Clone)]
pub struct ToolCallEvent {
    /// Type of the event (always ToolCall).
    pub event_type: EventType,
    /// The raw line that triggered this event.
    pub raw_line: String,
    /// Name of the tool that was called.
    pub tool_name: String,
}

/// Type of file change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileChangeType {
    /// A new file was created.
    Created,
    /// An existing file was modified.
    Modified,
    /// A file was deleted.
    Deleted,
    /// A file was read (not modified).
    Read,
}

/// Event for file changes.
#[derive(Debug, Clone)]
pub struct FileChangeEvent {
    /// Type of the event (always FileChange).
    pub event_type: EventType,
    /// The raw line that triggered this event.
    pub raw_line: String,
    /// Path to the file that was changed.
    pub file_path: String,
    /// Type of change (created, modified, deleted, read).
    pub change_type: FileChangeType,
}

/// Event for errors.
#[derive(Debug, Clone)]
pub struct ErrorEvent {
    /// Type of the event (always Error).
    pub event_type: EventType,
    /// The raw line that triggered this event.
    pub raw_line: String,
    /// The error message.
    pub error_message: String,
}

/// Event for warnings.
#[derive(Debug, Clone)]
pub struct WarningEvent {
    /// Type of the event (always Warning).
    pub event_type: EventType,
    /// The raw line that triggered this event.
    pub raw_line: String,
    /// The warning message.
    pub warning_message: String,
}

/// Union of all event types.
#[derive(Debug, Clone)]
pub enum ParsedEvent {
    /// A tool call event (e.g., Read, Write, Edit, Shell).
    ToolCall(ToolCallEvent),
    /// A file change event.
    FileChange(FileChangeEvent),
    /// An error event.
    Error(ErrorEvent),
    /// A warning event.
    Warning(WarningEvent),
}

impl ParsedEvent {
    /// Get the event type.
    pub fn event_type(&self) -> &EventType {
        match self {
            ParsedEvent::ToolCall(e) => &e.event_type,
            ParsedEvent::FileChange(e) => &e.event_type,
            ParsedEvent::Error(e) => &e.event_type,
            ParsedEvent::Warning(e) => &e.event_type,
        }
    }

    /// Get the raw line.
    pub fn raw_line(&self) -> &str {
        match self {
            ParsedEvent::ToolCall(e) => &e.raw_line,
            ParsedEvent::FileChange(e) => &e.raw_line,
            ParsedEvent::Error(e) => &e.raw_line,
            ParsedEvent::Warning(e) => &e.raw_line,
        }
    }
}

// ============================================================================
// Regex patterns
// ============================================================================

// Claude Code patterns
static CLAUDE_TOOL_CALL: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"Calling tool: (\w+)").expect("CLAUDE_TOOL_CALL regex is valid"));
static CLAUDE_FILE_WRITE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"Writing to: (.+)").expect("CLAUDE_FILE_WRITE regex is valid"));
static CLAUDE_FILE_READ: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"Reading: (.+)").expect("CLAUDE_FILE_READ regex is valid"));

// Cursor CLI patterns
static CURSOR_TOOL_CALL: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"⏺\s+(\w+)\(").expect("CURSOR_TOOL_CALL regex is valid"));
static CURSOR_FILE_EDITED: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^Edited\s+(.+)$").expect("CURSOR_FILE_EDITED regex is valid"));
static CURSOR_FILE_CREATED: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^Created\s+(.+)$").expect("CURSOR_FILE_CREATED regex is valid"));
static CURSOR_FILE_DELETED: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^Deleted\s+(.+)$").expect("CURSOR_FILE_DELETED regex is valid"));

// Aider patterns
static AIDER_APPLIED_EDIT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"Applied edit to (.+)").expect("AIDER_APPLIED_EDIT regex is valid"));
static AIDER_WROTE_FILE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^Wrote\s+(.+)$").expect("AIDER_WROTE_FILE regex is valid"));
static AIDER_ADDED_TO_CHAT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"Added (.+) to the chat").expect("AIDER_ADDED_TO_CHAT regex is valid")
});
static AIDER_COMMIT_MADE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"Commit ([a-f0-9]+)\s+(.+)").expect("AIDER_COMMIT_MADE regex is valid")
});

// Error patterns
static ERROR_PREFIX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(?:^|[\[\]\s])(?:Error|ERROR):\s*(.+)").expect("ERROR_PREFIX regex is valid")
});
static EXCEPTION_PREFIX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?:^|[\s])Exception:\s*(.+)").expect("EXCEPTION_PREFIX regex is valid")
});
static TRACEBACK: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^Traceback \(most recent call last\):").expect("TRACEBACK regex is valid")
});

// Warning patterns
static WARNING_PREFIX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(?:^|[\[\]\s])Warning:\s*(.+)").expect("WARNING_PREFIX regex is valid")
});
static DEPRECATION_WARNING: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"DeprecationWarning:\s*(.+)").expect("DEPRECATION_WARNING regex is valid")
});

/// Parse AI CLI output to detect tool calls, file operations, and events.
///
/// Supports multiple AI CLIs with different output formats.
pub struct OutputParser;

impl OutputParser {
    /// Create a new OutputParser.
    pub fn new() -> Self {
        Self
    }

    /// Parse a line of output and return any detected events.
    pub fn parse(&self, line: &str) -> Vec<ParsedEvent> {
        let mut events = Vec::new();

        // Skip empty lines
        if line.trim().is_empty() {
            return events;
        }

        // Claude Code patterns
        if let Some(caps) = CLAUDE_TOOL_CALL.captures(line) {
            events.push(ParsedEvent::ToolCall(ToolCallEvent {
                event_type: EventType::ToolCall,
                raw_line: line.to_string(),
                tool_name: caps[1].to_string(),
            }));
        }

        if let Some(caps) = CLAUDE_FILE_WRITE.captures(line) {
            events.push(ParsedEvent::FileChange(FileChangeEvent {
                event_type: EventType::FileChange,
                raw_line: line.to_string(),
                file_path: caps[1].trim().to_string(),
                change_type: FileChangeType::Modified,
            }));
        }

        if let Some(caps) = CLAUDE_FILE_READ.captures(line) {
            events.push(ParsedEvent::FileChange(FileChangeEvent {
                event_type: EventType::FileChange,
                raw_line: line.to_string(),
                file_path: caps[1].trim().to_string(),
                change_type: FileChangeType::Read,
            }));
        }

        // Cursor patterns
        if let Some(caps) = CURSOR_TOOL_CALL.captures(line) {
            events.push(ParsedEvent::ToolCall(ToolCallEvent {
                event_type: EventType::ToolCall,
                raw_line: line.to_string(),
                tool_name: caps[1].to_string(),
            }));
        }

        if let Some(caps) = CURSOR_FILE_EDITED.captures(line) {
            events.push(ParsedEvent::FileChange(FileChangeEvent {
                event_type: EventType::FileChange,
                raw_line: line.to_string(),
                file_path: caps[1].trim().to_string(),
                change_type: FileChangeType::Modified,
            }));
        }

        if let Some(caps) = CURSOR_FILE_CREATED.captures(line) {
            events.push(ParsedEvent::FileChange(FileChangeEvent {
                event_type: EventType::FileChange,
                raw_line: line.to_string(),
                file_path: caps[1].trim().to_string(),
                change_type: FileChangeType::Created,
            }));
        }

        if let Some(caps) = CURSOR_FILE_DELETED.captures(line) {
            events.push(ParsedEvent::FileChange(FileChangeEvent {
                event_type: EventType::FileChange,
                raw_line: line.to_string(),
                file_path: caps[1].trim().to_string(),
                change_type: FileChangeType::Deleted,
            }));
        }

        // Aider patterns
        if let Some(caps) = AIDER_APPLIED_EDIT.captures(line) {
            events.push(ParsedEvent::FileChange(FileChangeEvent {
                event_type: EventType::FileChange,
                raw_line: line.to_string(),
                file_path: caps[1].trim().to_string(),
                change_type: FileChangeType::Modified,
            }));
        }

        if let Some(caps) = AIDER_WROTE_FILE.captures(line) {
            events.push(ParsedEvent::FileChange(FileChangeEvent {
                event_type: EventType::FileChange,
                raw_line: line.to_string(),
                file_path: caps[1].trim().to_string(),
                change_type: FileChangeType::Modified,
            }));
        }

        if let Some(caps) = AIDER_ADDED_TO_CHAT.captures(line) {
            events.push(ParsedEvent::FileChange(FileChangeEvent {
                event_type: EventType::FileChange,
                raw_line: line.to_string(),
                file_path: caps[1].trim().to_string(),
                change_type: FileChangeType::Read,
            }));
        }

        if AIDER_COMMIT_MADE.is_match(line) {
            events.push(ParsedEvent::ToolCall(ToolCallEvent {
                event_type: EventType::ToolCall,
                raw_line: line.to_string(),
                tool_name: "git_commit".to_string(),
            }));
        }

        // Error patterns
        if let Some(caps) = ERROR_PREFIX.captures(line) {
            events.push(ParsedEvent::Error(ErrorEvent {
                event_type: EventType::Error,
                raw_line: line.to_string(),
                error_message: caps[1].to_string(),
            }));
        }

        if let Some(caps) = EXCEPTION_PREFIX.captures(line) {
            events.push(ParsedEvent::Error(ErrorEvent {
                event_type: EventType::Error,
                raw_line: line.to_string(),
                error_message: caps[1].to_string(),
            }));
        }

        if TRACEBACK.is_match(line) {
            events.push(ParsedEvent::Error(ErrorEvent {
                event_type: EventType::Error,
                raw_line: line.to_string(),
                error_message: "Python traceback detected".to_string(),
            }));
        }

        // Warning patterns
        if let Some(caps) = WARNING_PREFIX.captures(line) {
            events.push(ParsedEvent::Warning(WarningEvent {
                event_type: EventType::Warning,
                raw_line: line.to_string(),
                warning_message: caps[1].to_string(),
            }));
        }

        if let Some(caps) = DEPRECATION_WARNING.captures(line) {
            events.push(ParsedEvent::Warning(WarningEvent {
                event_type: EventType::Warning,
                raw_line: line.to_string(),
                warning_message: caps[1].to_string(),
            }));
        }

        events
    }
}

impl Default for OutputParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_line() {
        let parser = OutputParser::new();
        let events = parser.parse("");
        assert!(events.is_empty());
    }

    #[test]
    fn test_whitespace_only() {
        let parser = OutputParser::new();
        let events = parser.parse("   \t  ");
        assert!(events.is_empty());
    }

    #[test]
    fn test_no_match() {
        let parser = OutputParser::new();
        let events = parser.parse("Just some regular output");
        assert!(events.is_empty());
    }

    // Claude patterns
    #[test]
    fn test_claude_tool_call() {
        let parser = OutputParser::new();
        let events = parser.parse("Calling tool: write_file");
        assert_eq!(events.len(), 1);
        match &events[0] {
            ParsedEvent::ToolCall(e) => {
                assert_eq!(e.tool_name, "write_file");
            }
            _ => panic!("Expected ToolCall event"),
        }
    }

    #[test]
    fn test_claude_file_write() {
        let parser = OutputParser::new();
        let events = parser.parse("Writing to: src/main.rs");
        assert_eq!(events.len(), 1);
        match &events[0] {
            ParsedEvent::FileChange(e) => {
                assert_eq!(e.file_path, "src/main.rs");
                assert_eq!(e.change_type, FileChangeType::Modified);
            }
            _ => panic!("Expected FileChange event"),
        }
    }

    #[test]
    fn test_claude_file_read() {
        let parser = OutputParser::new();
        let events = parser.parse("Reading: src/lib.rs");
        assert_eq!(events.len(), 1);
        match &events[0] {
            ParsedEvent::FileChange(e) => {
                assert_eq!(e.file_path, "src/lib.rs");
                assert_eq!(e.change_type, FileChangeType::Read);
            }
            _ => panic!("Expected FileChange event"),
        }
    }

    // Cursor patterns
    #[test]
    fn test_cursor_tool_call() {
        let parser = OutputParser::new();
        let events = parser.parse("⏺ WriteFile(path=\"test.rs\")");
        assert_eq!(events.len(), 1);
        match &events[0] {
            ParsedEvent::ToolCall(e) => {
                assert_eq!(e.tool_name, "WriteFile");
            }
            _ => panic!("Expected ToolCall event"),
        }
    }

    #[test]
    fn test_cursor_file_edited() {
        let parser = OutputParser::new();
        let events = parser.parse("Edited src/main.rs");
        assert_eq!(events.len(), 1);
        match &events[0] {
            ParsedEvent::FileChange(e) => {
                assert_eq!(e.file_path, "src/main.rs");
                assert_eq!(e.change_type, FileChangeType::Modified);
            }
            _ => panic!("Expected FileChange event"),
        }
    }

    #[test]
    fn test_cursor_file_created() {
        let parser = OutputParser::new();
        let events = parser.parse("Created src/new_file.rs");
        assert_eq!(events.len(), 1);
        match &events[0] {
            ParsedEvent::FileChange(e) => {
                assert_eq!(e.file_path, "src/new_file.rs");
                assert_eq!(e.change_type, FileChangeType::Created);
            }
            _ => panic!("Expected FileChange event"),
        }
    }

    #[test]
    fn test_cursor_file_deleted() {
        let parser = OutputParser::new();
        let events = parser.parse("Deleted src/old_file.rs");
        assert_eq!(events.len(), 1);
        match &events[0] {
            ParsedEvent::FileChange(e) => {
                assert_eq!(e.file_path, "src/old_file.rs");
                assert_eq!(e.change_type, FileChangeType::Deleted);
            }
            _ => panic!("Expected FileChange event"),
        }
    }

    // Aider patterns
    #[test]
    fn test_aider_applied_edit() {
        let parser = OutputParser::new();
        let events = parser.parse("Applied edit to src/utils.rs");
        assert_eq!(events.len(), 1);
        match &events[0] {
            ParsedEvent::FileChange(e) => {
                assert_eq!(e.file_path, "src/utils.rs");
                assert_eq!(e.change_type, FileChangeType::Modified);
            }
            _ => panic!("Expected FileChange event"),
        }
    }

    #[test]
    fn test_aider_wrote_file() {
        let parser = OutputParser::new();
        let events = parser.parse("Wrote src/config.rs");
        assert_eq!(events.len(), 1);
        match &events[0] {
            ParsedEvent::FileChange(e) => {
                assert_eq!(e.file_path, "src/config.rs");
            }
            _ => panic!("Expected FileChange event"),
        }
    }

    #[test]
    fn test_aider_added_to_chat() {
        let parser = OutputParser::new();
        let events = parser.parse("Added README.md to the chat");
        assert_eq!(events.len(), 1);
        match &events[0] {
            ParsedEvent::FileChange(e) => {
                assert_eq!(e.file_path, "README.md");
                assert_eq!(e.change_type, FileChangeType::Read);
            }
            _ => panic!("Expected FileChange event"),
        }
    }

    #[test]
    fn test_aider_commit() {
        let parser = OutputParser::new();
        let events = parser.parse("Commit abc123f Added new feature");
        assert_eq!(events.len(), 1);
        match &events[0] {
            ParsedEvent::ToolCall(e) => {
                assert_eq!(e.tool_name, "git_commit");
            }
            _ => panic!("Expected ToolCall event"),
        }
    }

    // Error patterns
    #[test]
    fn test_error_prefix() {
        let parser = OutputParser::new();
        let events = parser.parse("Error: Something went wrong");
        assert_eq!(events.len(), 1);
        match &events[0] {
            ParsedEvent::Error(e) => {
                assert_eq!(e.error_message, "Something went wrong");
            }
            _ => panic!("Expected Error event"),
        }
    }

    #[test]
    fn test_exception() {
        let parser = OutputParser::new();
        let events = parser.parse("Exception: ValueError occurred");
        assert_eq!(events.len(), 1);
        match &events[0] {
            ParsedEvent::Error(e) => {
                assert_eq!(e.error_message, "ValueError occurred");
            }
            _ => panic!("Expected Error event"),
        }
    }

    #[test]
    fn test_traceback() {
        let parser = OutputParser::new();
        let events = parser.parse("Traceback (most recent call last):");
        assert_eq!(events.len(), 1);
        match &events[0] {
            ParsedEvent::Error(e) => {
                assert!(e.error_message.contains("traceback"));
            }
            _ => panic!("Expected Error event"),
        }
    }

    // Warning patterns
    #[test]
    fn test_warning_prefix() {
        let parser = OutputParser::new();
        let events = parser.parse("Warning: This might not work");
        assert_eq!(events.len(), 1);
        match &events[0] {
            ParsedEvent::Warning(e) => {
                assert_eq!(e.warning_message, "This might not work");
            }
            _ => panic!("Expected Warning event"),
        }
    }

    #[test]
    fn test_deprecation_warning() {
        let parser = OutputParser::new();
        let events = parser.parse("DeprecationWarning: Use new_function instead");
        assert_eq!(events.len(), 1);
        match &events[0] {
            ParsedEvent::Warning(e) => {
                assert!(e.warning_message.contains("new_function"));
            }
            _ => panic!("Expected Warning event"),
        }
    }

    #[test]
    fn test_parsed_event_methods() {
        let parser = OutputParser::new();
        let events = parser.parse("Calling tool: test_tool");
        assert!(!events.is_empty());
        let event = &events[0];
        assert_eq!(*event.event_type(), EventType::ToolCall);
        assert!(event.raw_line().contains("Calling tool"));
    }

    #[test]
    fn test_multiple_events_from_line() {
        let parser = OutputParser::new();
        // A line that matches both tool call and error (unlikely but possible)
        let events = parser.parse("Calling tool: handle_error Error: test failure");
        // Should have both ToolCall and Error
        assert!(events.len() >= 2);
    }
}

//! NDJSON stream parser for AI CLI output.
//!
//! Parses newline-delimited JSON (NDJSON) events from Cursor and Claude CLIs
//! when using `--output-format stream-json`.

use serde_json::Value;

/// Normalised stream event from any supported AI CLI.
#[derive(Debug, Clone, PartialEq)]
pub enum StreamEvent {
    /// System initialisation event.
    SystemInit {
        /// The model being used.
        model: Option<String>,
        /// The session ID.
        session_id: Option<String>,
    },
    /// User message (the prompt).
    UserMessage {
        /// The message text.
        text: String,
    },
    /// Assistant message (what the AI is saying/planning).
    AssistantMessage {
        /// The message text.
        text: String,
    },
    /// Tool call started.
    ToolStarted {
        /// Name of the tool.
        tool_name: String,
        /// Type of the tool.
        tool_type: ToolType,
        /// File path if applicable.
        path: Option<String>,
    },
    /// Tool call completed.
    ToolCompleted {
        /// Name of the tool.
        tool_name: String,
        /// Type of the tool.
        tool_type: ToolType,
        /// File path if applicable.
        path: Option<String>,
        /// Whether the tool call succeeded.
        success: bool,
        /// Number of lines read/written if applicable.
        lines: Option<u32>,
        /// File size if applicable.
        file_size: Option<u32>,
    },
    /// Session/iteration result.
    Result {
        /// Whether the session succeeded.
        success: bool,
        /// Duration in milliseconds.
        duration_ms: Option<u64>,
        /// Result text/summary.
        result_text: Option<String>,
    },
    /// Error event.
    Error {
        /// The error message.
        message: String,
    },
    /// Unknown event type (raw JSON preserved).
    Unknown {
        /// The event type string.
        event_type: String,
        /// The raw JSON.
        raw: String,
    },
}

/// Type of tool being used.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolType {
    /// File read operation.
    Read,
    /// File write operation.
    Write,
    /// File edit operation.
    Edit,
    /// File delete operation.
    Delete,
    /// Shell command execution.
    Command,
    /// Search operation (grep, glob, etc.).
    Search,
    /// Other tool type with custom name.
    Other(String),
}

impl std::fmt::Display for ToolType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ToolType::Read => write!(f, "Read"),
            ToolType::Write => write!(f, "Write"),
            ToolType::Edit => write!(f, "Edit"),
            ToolType::Delete => write!(f, "Delete"),
            ToolType::Command => write!(f, "Command"),
            ToolType::Search => write!(f, "Search"),
            ToolType::Other(name) => write!(f, "{}", name),
        }
    }
}

/// Which CLI format to parse.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CliFormat {
    /// Cursor CLI stream-json format.
    #[default]
    Cursor,
    /// Claude Code CLI stream-json format.
    Claude,
    /// Auto-detect from first event.
    Auto,
}

/// Parser for NDJSON stream output from AI CLIs.
pub struct StreamJsonParser {
    /// Which CLI format we're parsing.
    format: CliFormat,
    /// Detected format (after auto-detection).
    detected_format: Option<CliFormat>,
}

impl StreamJsonParser {
    /// Create a new parser with specified format.
    pub fn new(format: CliFormat) -> Self {
        Self {
            format,
            detected_format: None,
        }
    }

    /// Create a parser that auto-detects the format.
    pub fn auto_detect() -> Self {
        Self::new(CliFormat::Auto)
    }

    /// Get the detected or configured format.
    pub fn effective_format(&self) -> CliFormat {
        self.detected_format.unwrap_or(self.format)
    }

    /// Parse a single NDJSON line into a StreamEvent.
    pub fn parse_line(&mut self, line: &str) -> Option<StreamEvent> {
        let line = line.trim();
        if line.is_empty() {
            return None;
        }

        // Try to parse as JSON
        let json: Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => {
                // Not valid JSON, might be plain text output
                return None;
            }
        };

        // Auto-detect format from first event if needed
        if self.format == CliFormat::Auto && self.detected_format.is_none() {
            self.detected_format = Some(self.detect_format(&json));
        }

        let format = self.effective_format();
        match format {
            CliFormat::Cursor => self.parse_cursor_event(&json, line),
            CliFormat::Claude => self.parse_claude_event(&json, line),
            CliFormat::Auto => self.parse_cursor_event(&json, line), // Fallback
        }
    }

    /// Detect format from JSON structure.
    fn detect_format(&self, json: &Value) -> CliFormat {
        // Cursor uses "tool_call" with "subtype"
        // Claude uses "tool_use" and "tool_result"
        if json.get("tool_call").is_some() {
            return CliFormat::Cursor;
        }
        if json.get("type").and_then(|t| t.as_str()) == Some("tool_use") {
            return CliFormat::Claude;
        }
        if json.get("type").and_then(|t| t.as_str()) == Some("tool_result") {
            return CliFormat::Claude;
        }
        // Check for Cursor-specific fields
        if json.get("subtype").is_some() {
            return CliFormat::Cursor;
        }
        // Default to Cursor
        CliFormat::Cursor
    }

    /// Parse a Cursor CLI stream-json event.
    fn parse_cursor_event(&self, json: &Value, raw: &str) -> Option<StreamEvent> {
        let event_type = json.get("type")?.as_str()?;

        match event_type {
            "system" => {
                let model = json
                    .get("model")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let session_id = json
                    .get("session_id")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                Some(StreamEvent::SystemInit { model, session_id })
            }
            "user" => {
                let text = extract_message_text(json.get("message")?)?;
                Some(StreamEvent::UserMessage { text })
            }
            "assistant" => {
                let text = extract_message_text(json.get("message")?)?;
                Some(StreamEvent::AssistantMessage { text })
            }
            "tool_call" => {
                let subtype = json.get("subtype")?.as_str()?;
                let tool_call = json.get("tool_call")?;

                // Extract tool info from Cursor's nested structure
                let (tool_name, tool_type, path) = extract_cursor_tool_info(tool_call);

                match subtype {
                    "started" => Some(StreamEvent::ToolStarted {
                        tool_name,
                        tool_type,
                        path,
                    }),
                    "completed" => {
                        let (success, lines, file_size) = extract_cursor_tool_result(tool_call);
                        Some(StreamEvent::ToolCompleted {
                            tool_name,
                            tool_type,
                            path,
                            success,
                            lines,
                            file_size,
                        })
                    }
                    _ => Some(StreamEvent::Unknown {
                        event_type: format!("tool_call.{}", subtype),
                        raw: raw.to_string(),
                    }),
                }
            }
            "result" => {
                let subtype = json.get("subtype").and_then(|v| v.as_str());
                let success = subtype == Some("success")
                    || json.get("is_error").and_then(|v| v.as_bool()) == Some(false);
                let duration_ms = json.get("duration_ms").and_then(|v| v.as_u64());
                let result_text = json
                    .get("result")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                Some(StreamEvent::Result {
                    success,
                    duration_ms,
                    result_text,
                })
            }
            _ => Some(StreamEvent::Unknown {
                event_type: event_type.to_string(),
                raw: raw.to_string(),
            }),
        }
    }

    /// Parse a Claude CLI stream-json event.
    fn parse_claude_event(&self, json: &Value, raw: &str) -> Option<StreamEvent> {
        let event_type = json.get("type")?.as_str()?;

        match event_type {
            "system" => {
                let model = json
                    .get("model")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let session_id = json
                    .get("session_id")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                Some(StreamEvent::SystemInit { model, session_id })
            }
            "user" => {
                let text = extract_message_text(json.get("message")?)?;
                Some(StreamEvent::UserMessage { text })
            }
            "assistant" => {
                let text = extract_message_text(json.get("message")?)?;
                Some(StreamEvent::AssistantMessage { text })
            }
            "tool_use" => {
                let tool_name = json
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string();
                let tool_type = classify_tool_name(&tool_name);
                let path = json
                    .get("input")
                    .and_then(|i| i.get("path"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                Some(StreamEvent::ToolStarted {
                    tool_name,
                    tool_type,
                    path,
                })
            }
            "tool_result" => {
                // Claude doesn't include tool name in result, use generic
                Some(StreamEvent::ToolCompleted {
                    tool_name: "tool".to_string(),
                    tool_type: ToolType::Other("tool".to_string()),
                    path: None,
                    success: true,
                    lines: None,
                    file_size: None,
                })
            }
            "result" => {
                let subtype = json.get("subtype").and_then(|v| v.as_str());
                let success = subtype == Some("success");
                let duration_ms = json.get("duration_ms").and_then(|v| v.as_u64());
                let result_text = json
                    .get("result")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                Some(StreamEvent::Result {
                    success,
                    duration_ms,
                    result_text,
                })
            }
            "error" => {
                let message = json
                    .get("message")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown error")
                    .to_string();
                Some(StreamEvent::Error { message })
            }
            _ => Some(StreamEvent::Unknown {
                event_type: event_type.to_string(),
                raw: raw.to_string(),
            }),
        }
    }
}

impl Default for StreamJsonParser {
    fn default() -> Self {
        Self::auto_detect()
    }
}

/// Extract text content from a message object.
/// Returns the extracted text, or an empty string if the message has no text
/// (e.g., contains only tool_use references).
fn extract_message_text(message: &Value) -> Option<String> {
    // Try content array format: {"content": [{"type": "text", "text": "..."}]}
    if let Some(content) = message.get("content") {
        if let Some(arr) = content.as_array() {
            let mut texts = Vec::new();
            for item in arr {
                if let Some(text) = item.get("text").and_then(|v| v.as_str()) {
                    texts.push(text);
                }
            }
            if !texts.is_empty() {
                return Some(texts.join(""));
            }
            // Content array exists but has no text items (e.g., only tool_use)
            // Return empty string to indicate we parsed successfully but found no text
            if !arr.is_empty() {
                return Some(String::new());
            }
        }
        // Also try content as string directly
        if let Some(text) = content.as_str() {
            return Some(text.to_string());
        }
    }
    // Try direct text field
    if let Some(text) = message.get("text").and_then(|v| v.as_str()) {
        return Some(text.to_string());
    }
    None
}

/// Extract tool info from Cursor's nested tool_call structure.
fn extract_cursor_tool_info(tool_call: &Value) -> (String, ToolType, Option<String>) {
    // Cursor uses: {"readToolCall": {...}}, {"writeToolCall": {...}}, etc.
    let tool_types = [
        ("readToolCall", ToolType::Read, "Read"),
        ("writeToolCall", ToolType::Write, "Write"),
        ("editToolCall", ToolType::Edit, "Edit"),
        ("deleteToolCall", ToolType::Delete, "Delete"),
        ("bashToolCall", ToolType::Command, "Bash"),
        ("searchToolCall", ToolType::Search, "Search"),
        ("grepToolCall", ToolType::Search, "Grep"),
        ("globToolCall", ToolType::Search, "Glob"),
    ];

    for (key, tool_type, name) in tool_types {
        if let Some(inner) = tool_call.get(key) {
            let path = inner
                .get("args")
                .and_then(|a| a.get("path"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            return (name.to_string(), tool_type, path);
        }
    }

    // Try generic function format
    if let Some(func) = tool_call.get("function") {
        let name = func
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("Thinking...")
            .to_string();
        let tool_type = classify_tool_name(&name);
        return (name, tool_type, None);
    }

    (
        "Thinking...".to_string(),
        ToolType::Other("Thinking...".to_string()),
        None,
    )
}

/// Extract result info from Cursor's tool_call completion.
fn extract_cursor_tool_result(tool_call: &Value) -> (bool, Option<u32>, Option<u32>) {
    // Look for result.success in any of the tool call types
    for key in [
        "readToolCall",
        "writeToolCall",
        "editToolCall",
        "deleteToolCall",
    ] {
        if let Some(inner) = tool_call.get(key) {
            if let Some(result) = inner.get("result") {
                let success = result.get("success").is_some();
                let lines = result
                    .get("success")
                    .and_then(|s| s.get("totalLines").or_else(|| s.get("linesCreated")))
                    .and_then(|v| v.as_u64())
                    .map(|v| v as u32);
                let file_size = result
                    .get("success")
                    .and_then(|s| s.get("fileSize"))
                    .and_then(|v| v.as_u64())
                    .map(|v| v as u32);
                return (success, lines, file_size);
            }
        }
    }
    (true, None, None)
}

/// Classify a tool name into a ToolType.
fn classify_tool_name(name: &str) -> ToolType {
    let lower = name.to_lowercase();
    if lower.contains("read") {
        ToolType::Read
    } else if lower.contains("write") {
        ToolType::Write
    } else if lower.contains("edit") {
        ToolType::Edit
    } else if lower.contains("delete") || lower.contains("remove") {
        ToolType::Delete
    } else if lower.contains("bash") || lower.contains("command") || lower.contains("exec") {
        ToolType::Command
    } else if lower.contains("search") || lower.contains("grep") || lower.contains("glob") {
        ToolType::Search
    } else {
        ToolType::Other(name.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_empty_line() {
        let mut parser = StreamJsonParser::auto_detect();
        assert!(parser.parse_line("").is_none());
        assert!(parser.parse_line("   ").is_none());
    }

    #[test]
    fn test_parse_invalid_json() {
        let mut parser = StreamJsonParser::auto_detect();
        assert!(parser.parse_line("not valid json").is_none());
    }

    #[test]
    fn test_parse_cursor_system_init() {
        let mut parser = StreamJsonParser::new(CliFormat::Cursor);
        let event = parser
            .parse_line(r#"{"type":"system","subtype":"init","model":"Claude 4 Sonnet","session_id":"abc123"}"#)
            .unwrap();
        match event {
            StreamEvent::SystemInit { model, session_id } => {
                assert_eq!(model, Some("Claude 4 Sonnet".to_string()));
                assert_eq!(session_id, Some("abc123".to_string()));
            }
            _ => panic!("Expected SystemInit"),
        }
    }

    #[test]
    fn test_parse_cursor_assistant_message() {
        let mut parser = StreamJsonParser::new(CliFormat::Cursor);
        let event = parser
            .parse_line(r#"{"type":"assistant","message":{"role":"assistant","content":[{"type":"text","text":"I'll read the file"}]}}"#)
            .unwrap();
        match event {
            StreamEvent::AssistantMessage { text } => {
                assert_eq!(text, "I'll read the file");
            }
            _ => panic!("Expected AssistantMessage"),
        }
    }

    #[test]
    fn test_parse_cursor_tool_started() {
        let mut parser = StreamJsonParser::new(CliFormat::Cursor);
        let event = parser
            .parse_line(r#"{"type":"tool_call","subtype":"started","tool_call":{"readToolCall":{"args":{"path":"src/main.rs"}}}}"#)
            .unwrap();
        match event {
            StreamEvent::ToolStarted {
                tool_name,
                tool_type,
                path,
            } => {
                assert_eq!(tool_name, "Read");
                assert_eq!(tool_type, ToolType::Read);
                assert_eq!(path, Some("src/main.rs".to_string()));
            }
            _ => panic!("Expected ToolStarted"),
        }
    }

    #[test]
    fn test_parse_cursor_tool_completed() {
        let mut parser = StreamJsonParser::new(CliFormat::Cursor);
        let event = parser
            .parse_line(r#"{"type":"tool_call","subtype":"completed","tool_call":{"readToolCall":{"args":{"path":"src/main.rs"},"result":{"success":{"totalLines":42}}}}}"#)
            .unwrap();
        match event {
            StreamEvent::ToolCompleted {
                tool_name,
                tool_type,
                path,
                success,
                lines,
                ..
            } => {
                assert_eq!(tool_name, "Read");
                assert_eq!(tool_type, ToolType::Read);
                assert_eq!(path, Some("src/main.rs".to_string()));
                assert!(success);
                assert_eq!(lines, Some(42));
            }
            _ => panic!("Expected ToolCompleted"),
        }
    }

    #[test]
    fn test_parse_cursor_write_tool_completed() {
        let mut parser = StreamJsonParser::new(CliFormat::Cursor);
        let event = parser
            .parse_line(r#"{"type":"tool_call","subtype":"completed","tool_call":{"writeToolCall":{"args":{"path":"test.rs"},"result":{"success":{"linesCreated":19,"fileSize":942}}}}}"#)
            .unwrap();
        match event {
            StreamEvent::ToolCompleted {
                tool_name,
                tool_type,
                lines,
                file_size,
                ..
            } => {
                assert_eq!(tool_name, "Write");
                assert_eq!(tool_type, ToolType::Write);
                assert_eq!(lines, Some(19));
                assert_eq!(file_size, Some(942));
            }
            _ => panic!("Expected ToolCompleted"),
        }
    }

    #[test]
    fn test_parse_cursor_result() {
        let mut parser = StreamJsonParser::new(CliFormat::Cursor);
        let event = parser
            .parse_line(r#"{"type":"result","subtype":"success","is_error":false,"duration_ms":5234,"result":"Done!"}"#)
            .unwrap();
        match event {
            StreamEvent::Result {
                success,
                duration_ms,
                result_text,
            } => {
                assert!(success);
                assert_eq!(duration_ms, Some(5234));
                assert_eq!(result_text, Some("Done!".to_string()));
            }
            _ => panic!("Expected Result"),
        }
    }

    #[test]
    fn test_parse_claude_assistant_message() {
        let mut parser = StreamJsonParser::new(CliFormat::Claude);
        let event = parser
            .parse_line(r#"{"type":"assistant","message":{"role":"assistant","content":[{"type":"text","text":"Let me help you"}]}}"#)
            .unwrap();
        match event {
            StreamEvent::AssistantMessage { text } => {
                assert_eq!(text, "Let me help you");
            }
            _ => panic!("Expected AssistantMessage"),
        }
    }

    #[test]
    fn test_parse_claude_tool_use() {
        let mut parser = StreamJsonParser::new(CliFormat::Claude);
        let event = parser
            .parse_line(r#"{"type":"tool_use","name":"Read","input":{"path":"config.rs"}}"#)
            .unwrap();
        match event {
            StreamEvent::ToolStarted {
                tool_name,
                tool_type,
                path,
            } => {
                assert_eq!(tool_name, "Read");
                assert_eq!(tool_type, ToolType::Read);
                assert_eq!(path, Some("config.rs".to_string()));
            }
            _ => panic!("Expected ToolStarted"),
        }
    }

    #[test]
    fn test_auto_detect_cursor() {
        let mut parser = StreamJsonParser::auto_detect();
        parser.parse_line(
            r#"{"type":"tool_call","subtype":"started","tool_call":{"readToolCall":{}}}"#,
        );
        assert_eq!(parser.effective_format(), CliFormat::Cursor);
    }

    #[test]
    fn test_auto_detect_claude() {
        let mut parser = StreamJsonParser::auto_detect();
        parser.parse_line(r#"{"type":"tool_use","name":"Read","input":{}}"#);
        assert_eq!(parser.effective_format(), CliFormat::Claude);
    }

    #[test]
    fn test_tool_type_display() {
        assert_eq!(format!("{}", ToolType::Read), "Read");
        assert_eq!(format!("{}", ToolType::Write), "Write");
        assert_eq!(
            format!("{}", ToolType::Other("Custom".to_string())),
            "Custom"
        );
    }

    #[test]
    fn test_classify_tool_name() {
        assert_eq!(classify_tool_name("ReadFile"), ToolType::Read);
        assert_eq!(classify_tool_name("WriteFile"), ToolType::Write);
        assert_eq!(classify_tool_name("EditFile"), ToolType::Edit);
        assert_eq!(classify_tool_name("DeleteFile"), ToolType::Delete);
        assert_eq!(classify_tool_name("Bash"), ToolType::Command);
        assert_eq!(classify_tool_name("Grep"), ToolType::Search);
    }

    #[test]
    fn test_unknown_event() {
        let mut parser = StreamJsonParser::new(CliFormat::Cursor);
        let event = parser
            .parse_line(r#"{"type":"future_event","data":"something"}"#)
            .unwrap();
        match event {
            StreamEvent::Unknown { event_type, .. } => {
                assert_eq!(event_type, "future_event");
            }
            _ => panic!("Expected Unknown"),
        }
    }

    #[test]
    fn test_parse_user_message_with_tool_use_only() {
        // User messages that only contain tool_use references (no text) should
        // return a UserMessage with empty text, not None
        let mut parser = StreamJsonParser::new(CliFormat::Claude);
        let event = parser
            .parse_line(r#"{"type":"user","message":{"role":"user","content":[{"tool_use_id":"toolu_123","type":"tool_result","content":"result"}]}}"#)
            .unwrap();
        match event {
            StreamEvent::UserMessage { text } => {
                assert!(text.is_empty(), "Expected empty text for tool-use-only message");
            }
            _ => panic!("Expected UserMessage"),
        }
    }

    #[test]
    fn test_parse_assistant_message_with_mixed_content() {
        // Assistant messages with both text and tool_use should extract only the text
        let mut parser = StreamJsonParser::new(CliFormat::Claude);
        let event = parser
            .parse_line(r#"{"type":"assistant","message":{"role":"assistant","content":[{"type":"text","text":"Let me help"},{"type":"tool_use","id":"abc"}]}}"#)
            .unwrap();
        match event {
            StreamEvent::AssistantMessage { text } => {
                assert_eq!(text, "Let me help");
            }
            _ => panic!("Expected AssistantMessage"),
        }
    }
}

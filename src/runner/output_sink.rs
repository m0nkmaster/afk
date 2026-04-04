//! Unified output sink trait for streaming AI CLI output.
//!
//! This module provides a single streaming implementation shared by both
//! the console (non-TUI) and TUI code paths, fixing the NDJSON bug where
//! the TUI path would display raw JSON lines that the console path correctly suppressed.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::Arc;

use crate::parser::{StreamEvent, ToolType};
use crate::tui::TuiEvent;

use super::make_path_relative;
use super::output_handler::OutputHandler;

/// Trait for receiving streaming output from AI CLI subprocesses.
///
/// Both the console and TUI rendering paths implement this trait,
/// allowing `stream_subprocess_output` to work with either.
pub trait OutputSink {
    /// Display a line of plain text output.
    fn on_line(&mut self, line: &str);
    /// Handle a parsed NDJSON stream event.
    fn on_stream_event(&mut self, event: &StreamEvent);
    /// Signal that a completion marker was detected.
    fn on_completion(&mut self);
    /// Display a warning message.
    fn on_warning(&mut self, msg: &str);
    /// Check if text contains a completion signal.
    fn contains_completion_signal(&self, text: &str) -> bool;
    /// Check if the user has requested an interrupt.
    fn is_interrupted(&self) -> bool;
}

// ---------------------------------------------------------------------------
// Console (non-TUI) sink
// ---------------------------------------------------------------------------

/// Sink that writes to the console via `OutputHandler`.
/// Optionally forwards TUI events when a sender is present.
pub struct ConsoleSink<'a> {
    pub output: &'a mut OutputHandler,
    pub tui_sender: Option<&'a Sender<TuiEvent>>,
}

impl OutputSink for ConsoleSink<'_> {
    fn on_line(&mut self, line: &str) {
        self.output.stream_line(&format!("{line}\n"));
    }

    fn on_stream_event(&mut self, event: &StreamEvent) {
        let (display, tui_event) = stream_event_to_display(event);

        if let (Some(sender), Some(tui_event)) = (self.tui_sender, tui_event) {
            let _ = sender.send(tui_event);
        }

        if let Some(display) = display {
            self.output.stream_line(&format!("{display}\n"));
        }
    }

    fn on_completion(&mut self) {
        self.output.completion_detected();
    }

    fn on_warning(&mut self, msg: &str) {
        self.output.warning(msg);
    }

    fn contains_completion_signal(&self, text: &str) -> bool {
        self.output.contains_completion_signal(text)
    }

    fn is_interrupted(&self) -> bool {
        false // Console path doesn't support TUI interrupts
    }
}

// ---------------------------------------------------------------------------
// TUI sink
// ---------------------------------------------------------------------------

/// Sink that sends events to the TUI via an `mpsc::Sender`.
pub struct TuiSink {
    pub tx: Sender<TuiEvent>,
    pub interrupted: Arc<AtomicBool>,
}

impl OutputSink for TuiSink {
    fn on_line(&mut self, line: &str) {
        let _ = self.tx.send(TuiEvent::OutputLine(line.to_string()));

        // Track tool calls from output patterns (plain text mode)
        if line.contains("antml:invoke") || line.contains("<tool_call>") {
            let _ = self.tx.send(TuiEvent::ToolCall("tool".to_string()));
        }
    }

    fn on_stream_event(&mut self, event: &StreamEvent) {
        use crate::parser::StreamEvent::*;

        // Completion signal detection is handled by stream_subprocess_output,
        // which calls on_completion() and triggers the kill. No need to check here.

        match event {
            AssistantMessage { text } => {
                if text.is_empty() {
                    return;
                }
                let display_text = if text.len() > 200 {
                    format!("{}...", &text[..197])
                } else {
                    text.clone()
                };
                let _ = self.tx.send(TuiEvent::OutputLine(display_text));
            }
            ToolStarted {
                tool_name,
                tool_type,
                path,
            } => {
                let path_str = path
                    .as_ref()
                    .map(|p| format!(" {}", make_path_relative(p)))
                    .unwrap_or_default();
                let _ = self
                    .tx
                    .send(TuiEvent::OutputLine(format!("→ {}{}", tool_type, path_str)));
                let _ = self.tx.send(TuiEvent::ToolCall(tool_name.clone()));
            }
            ToolCompleted {
                tool_type,
                path,
                success,
                lines,
                ..
            } => {
                let status = if *success { "✓" } else { "✗" };
                let lines_str = lines.map(|l| format!(" ({} lines)", l)).unwrap_or_default();
                let path_str = path
                    .as_ref()
                    .map(|p| format!(" {}", make_path_relative(p)))
                    .unwrap_or_default();
                let _ = self.tx.send(TuiEvent::OutputLine(format!(
                    "{} {}{}{}",
                    status, tool_type, path_str, lines_str
                )));

                if let Some(p) = path {
                    let change_type = match tool_type {
                        ToolType::Read => "read",
                        ToolType::Write => "created",
                        ToolType::Edit => "modified",
                        ToolType::Delete => "deleted",
                        _ => "modified",
                    };
                    let _ = self.tx.send(TuiEvent::FileChange {
                        path: make_path_relative(p).to_string(),
                        change_type: change_type.to_string(),
                    });
                }
            }
            Result {
                success,
                duration_ms,
                ..
            } => {
                let status = if *success {
                    "✓ Complete"
                } else {
                    "✗ Failed"
                };
                let duration_str = duration_ms
                    .map(|ms| format!(" ({:.1}s)", ms as f64 / 1000.0))
                    .unwrap_or_default();
                let _ = self
                    .tx
                    .send(TuiEvent::OutputLine(format!("{}{}", status, duration_str)));
            }
            Error { message } => {
                let _ = self.tx.send(TuiEvent::Error(message.clone()));
            }
            SystemInit { model, .. } => {
                if let Some(m) = model {
                    let _ = self
                        .tx
                        .send(TuiEvent::OutputLine(format!("◉ Model: {}", m)));
                }
            }
            UserMessage { .. } | Unknown { .. } => {}
        }
    }

    fn on_completion(&mut self) {
        let _ = self.tx.send(TuiEvent::OutputLine(
            "✓ Completion signal detected".to_string(),
        ));
    }

    fn on_warning(&mut self, msg: &str) {
        let _ = self.tx.send(TuiEvent::Warning(format!("Warning: {msg}")));
    }

    fn contains_completion_signal(&self, text: &str) -> bool {
        text.contains("<promise>COMPLETE</promise>")
            || text.contains("AFK_COMPLETE")
            || text.contains("AFK_STOP")
    }

    fn is_interrupted(&self) -> bool {
        self.interrupted.load(Ordering::SeqCst)
    }
}

// ---------------------------------------------------------------------------
// Unified streaming function
// ---------------------------------------------------------------------------

/// Result of streaming subprocess output.
pub struct StreamResult {
    /// Whether a completion signal was detected.
    pub completion_detected: bool,
    /// Whether the user interrupted (TUI Q press).
    pub user_interrupted: bool,
}

/// Stream stdout from an AI CLI subprocess through an `OutputSink`.
///
/// This is the single implementation of the NDJSON + plain-text streaming loop,
/// shared by both the console and TUI paths. When the parser returns `None` for
/// a line, JSON-shaped lines are silently suppressed (fixing the TUI bug where
/// raw NDJSON was forwarded to the display).
pub fn stream_subprocess_output(
    reader: Box<dyn std::io::BufRead>,
    kill_fn: &mut dyn FnMut(),
    parser: &mut Option<crate::parser::StreamJsonParser>,
    sink: &mut dyn OutputSink,
    output_buf: &mut String,
) -> StreamResult {
    use std::io::BufRead;

    let mut completion_detected = false;
    let mut user_interrupted = false;

    for line in reader.lines() {
        // Check for user interrupt
        if sink.is_interrupted() {
            user_interrupted = true;
            kill_fn();
            break;
        }

        match line {
            Ok(line) => {
                if let Some(ref mut p) = parser {
                    // NDJSON mode
                    if let Some(event) = p.parse_line(&line) {
                        // Check for completion signal in assistant messages
                        if let StreamEvent::AssistantMessage { ref text } = event {
                            if sink.contains_completion_signal(text) {
                                completion_detected = true;
                                sink.on_completion();
                                kill_fn();
                                break;
                            }
                        }

                        sink.on_stream_event(&event);
                    } else {
                        // Parser returned None — decide whether to display or suppress.
                        // JSON-shaped lines are suppressed (they're NDJSON metadata);
                        // plain text is displayed (fallback for CLIs without stream-json).
                        let is_json =
                            line.trim_start().starts_with('{') && line.trim_end().ends_with('}');

                        if is_json {
                            // Silently skip, but still check for embedded completion signals
                            if sink.contains_completion_signal(&line) {
                                completion_detected = true;
                                sink.on_completion();
                                kill_fn();
                                break;
                            }
                        } else {
                            // Plain text fallback — display as-is
                            sink.on_line(&line);
                            if sink.contains_completion_signal(&line) {
                                completion_detected = true;
                                sink.on_completion();
                                kill_fn();
                                break;
                            }
                        }
                    }
                } else {
                    // Plain text mode — display every line
                    sink.on_line(&line);
                    if sink.contains_completion_signal(&line) {
                        completion_detected = true;
                        sink.on_completion();
                        kill_fn();
                        break;
                    }
                }

                output_buf.push_str(&line);
                output_buf.push('\n');
            }
            Err(e) => {
                sink.on_warning(&format!("Error reading output: {e}"));
                break;
            }
        }
    }

    StreamResult {
        completion_detected,
        user_interrupted,
    }
}

// ---------------------------------------------------------------------------
// Shared helper for console event display (extracted from iteration.rs)
// ---------------------------------------------------------------------------

/// Convert a StreamEvent to a display string and optional TuiEvent.
///
/// Used by `ConsoleSink` — the TUI sink handles events directly.
fn stream_event_to_display(event: &StreamEvent) -> (Option<String>, Option<TuiEvent>) {
    match event {
        StreamEvent::SystemInit { model, .. } => {
            let display = model
                .as_ref()
                .map(|m| format!("\x1b[2m◉ Model: {}\x1b[0m", m));
            (display, None)
        }
        StreamEvent::UserMessage { .. } => (None, None),
        StreamEvent::AssistantMessage { text } => {
            if text.is_empty() {
                return (None, None);
            }
            let display_text = if text.len() > 200 {
                format!("{}...", &text[..197])
            } else {
                text.clone()
            };
            let display = format!("\x1b[37m{}\x1b[0m", display_text);
            let tui_event = TuiEvent::OutputLine(text.clone());
            (Some(display), Some(tui_event))
        }
        StreamEvent::ToolStarted {
            tool_name,
            tool_type,
            path,
        } => {
            let path_str = path
                .as_ref()
                .map(|p| format!(" {}", make_path_relative(p)))
                .unwrap_or_default();
            let display = format!("\x1b[33m→ {}{}\x1b[0m", tool_type, path_str);
            let tui_event = TuiEvent::ToolCall(tool_name.clone());
            (Some(display), Some(tui_event))
        }
        StreamEvent::ToolCompleted {
            tool_type,
            path,
            success,
            lines,
            ..
        } => {
            let status = if *success { "✓" } else { "✗" };
            let lines_str = lines.map(|l| format!(" ({} lines)", l)).unwrap_or_default();
            let path_str = path
                .as_ref()
                .map(|p| format!(" {}", make_path_relative(p)))
                .unwrap_or_default();
            let colour = if *success { "\x1b[32m" } else { "\x1b[31m" };
            let display = format!(
                "{}{} {}{}{}\x1b[0m",
                colour, status, tool_type, path_str, lines_str
            );

            let tui_event = path.as_ref().map(|p| {
                let change_type = match tool_type {
                    ToolType::Read => "read",
                    ToolType::Write => "created",
                    ToolType::Edit => "modified",
                    ToolType::Delete => "deleted",
                    _ => "modified",
                };
                TuiEvent::FileChange {
                    path: make_path_relative(p).to_string(),
                    change_type: change_type.to_string(),
                }
            });

            (Some(display), tui_event)
        }
        StreamEvent::Result {
            success,
            duration_ms,
            ..
        } => {
            let status = if *success {
                "✓ Complete"
            } else {
                "✗ Failed"
            };
            let duration_str = duration_ms
                .map(|ms| format!(" ({:.1}s)", ms as f64 / 1000.0))
                .unwrap_or_default();
            let colour = if *success { "\x1b[32m" } else { "\x1b[31m" };
            let display = format!("{}{}{}\x1b[0m", colour, status, duration_str);

            let tui_event = duration_ms.map(|ms| TuiEvent::IterationComplete {
                duration_secs: ms as f64 / 1000.0,
            });

            (Some(display), tui_event)
        }
        StreamEvent::Error { message } => {
            let display = format!("\x1b[31m✗ Error: {}\x1b[0m", message);
            let tui_event = TuiEvent::Error(message.clone());
            (Some(display), Some(tui_event))
        }
        StreamEvent::Unknown { .. } => (None, None),
    }
}

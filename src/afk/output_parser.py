"""Output parsing for AI CLI tool output streams.

This module provides event types and data structures for parsing output
from AI coding tools (Claude Code, Cursor, Aider, etc.) to detect tool
calls, file changes, errors, and warnings.
"""

from __future__ import annotations

from dataclasses import dataclass
from enum import Enum


class EventType(Enum):
    """Types of events that can be detected in AI CLI output.

    Used to categorise parsed events for downstream processing
    by the MetricsCollector.
    """

    TOOL_CALL = "tool_call"
    """A tool was invoked by the AI (e.g., write_file, execute_command)."""

    FILE_CHANGE = "file_change"
    """A file was modified, created, or deleted."""

    ERROR = "error"
    """An error occurred during processing."""

    WARNING = "warning"
    """A warning was emitted during processing."""


@dataclass
class Event:
    """Base event dataclass for all parsed output events.

    All event types share these common fields. Specific event types
    extend this with additional relevant fields.
    """

    event_type: EventType
    """The type of event that was detected."""

    raw_line: str
    """The original line from the output stream that triggered this event."""


@dataclass
class ToolCallEvent(Event):
    """Event representing an AI tool call.

    Emitted when the parser detects that the AI invoked a tool,
    such as writing a file, reading content, or executing a command.
    """

    tool_name: str
    """Name of the tool that was called (e.g., 'write_file', 'Shell')."""


@dataclass
class FileChangeEvent(Event):
    """Event representing a file system change.

    Emitted when the parser detects a file being modified, created,
    or deleted by the AI.
    """

    file_path: str
    """Path to the file that was changed."""

    change_type: str
    """Type of change: 'modified', 'created', or 'deleted'."""


@dataclass
class ErrorEvent(Event):
    """Event representing an error in the output.

    Emitted when the parser detects error patterns such as exceptions,
    tracebacks, or explicit error messages.
    """

    error_message: str
    """The error message or description."""


@dataclass
class WarningEvent(Event):
    """Event representing a warning in the output.

    Emitted when the parser detects warning patterns such as deprecation
    notices, linter warnings, or other advisory messages.
    """

    warning_message: str
    """The warning message or description."""

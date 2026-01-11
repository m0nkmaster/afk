"""Output parsing for AI CLI tool output streams.

This module provides event types and data structures for parsing output
from AI coding tools (Claude Code, Cursor, Aider, etc.) to detect tool
calls, file changes, errors, and warnings.
"""

from __future__ import annotations

import re
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


class OutputParser:
    """Parse AI CLI output to detect tool calls, file operations, and events.

    Supports multiple AI CLIs with different output formats. Currently
    implements Claude Code and Cursor pattern detection.
    """

    # Claude Code output patterns
    _CLAUDE_TOOL_CALL = re.compile(r"Calling tool: (\w+)")
    _CLAUDE_FILE_WRITE = re.compile(r"Writing to: (.+)")
    _CLAUDE_FILE_READ = re.compile(r"Reading: (.+)")

    # Cursor CLI output patterns
    # Tool calls are prefixed with ⏺ (record symbol) followed by ToolName(params)
    _CURSOR_TOOL_CALL = re.compile(r"⏺\s+(\w+)\(")
    # File operations: "Edited path", "Created path", "Deleted path"
    _CURSOR_FILE_EDITED = re.compile(r"^Edited\s+(.+)$")
    _CURSOR_FILE_CREATED = re.compile(r"^Created\s+(.+)$")
    _CURSOR_FILE_DELETED = re.compile(r"^Deleted\s+(.+)$")

    # Error patterns - detect various error indicators
    # Generic Error: prefix (case insensitive)
    _ERROR_PREFIX = re.compile(r"(?:^|[\[\]\s])(?:Error|ERROR):\s*(.+)", re.IGNORECASE)
    # Exception: prefix
    _EXCEPTION_PREFIX = re.compile(r"(?:^|[\[\]\s])Exception:\s*(.+)")
    # Python Traceback header
    _TRACEBACK_HEADER = re.compile(r"Traceback \(most recent call last\):")
    # Python exception types (e.g., ValueError: message)
    _PYTHON_EXCEPTION = re.compile(
        r"([A-Z][a-zA-Z]*(?:Error|Exception)):\s*(.+)"
    )

    # Warning patterns - detect various warning indicators
    # Generic Warning: prefix (case insensitive)
    _WARNING_PREFIX = re.compile(r"(?:^|[\[\]\s])(?:Warning|WARNING):\s*(.+)", re.IGNORECASE)
    # Python warning types (e.g., DeprecationWarning: message)
    _PYTHON_WARNING = re.compile(
        r"([A-Z][a-zA-Z]*Warning):\s*(.+)"
    )

    def parse(self, line: str) -> list[Event]:
        """Parse a line of AI output and return detected events.

        Args:
            line: A single line from the AI CLI output stream.

        Returns:
            List of Event objects detected in the line. Empty list if
            no patterns match.
        """
        if not line:
            return []

        events: list[Event] = []

        # Check for Claude Code tool call pattern
        if match := self._CLAUDE_TOOL_CALL.search(line):
            events.append(
                ToolCallEvent(
                    event_type=EventType.TOOL_CALL,
                    raw_line=line,
                    tool_name=match.group(1),
                )
            )

        # Check for Claude Code file write pattern
        if match := self._CLAUDE_FILE_WRITE.search(line):
            events.append(
                FileChangeEvent(
                    event_type=EventType.FILE_CHANGE,
                    raw_line=line,
                    file_path=match.group(1).strip(),
                    change_type="modified",
                )
            )

        # Check for Claude Code file read pattern
        if match := self._CLAUDE_FILE_READ.search(line):
            events.append(
                FileChangeEvent(
                    event_type=EventType.FILE_CHANGE,
                    raw_line=line,
                    file_path=match.group(1).strip(),
                    change_type="read",
                )
            )

        # Check for Cursor tool call pattern
        if match := self._CURSOR_TOOL_CALL.search(line):
            events.append(
                ToolCallEvent(
                    event_type=EventType.TOOL_CALL,
                    raw_line=line,
                    tool_name=match.group(1),
                )
            )

        # Check for Cursor file edited pattern
        if match := self._CURSOR_FILE_EDITED.search(line):
            events.append(
                FileChangeEvent(
                    event_type=EventType.FILE_CHANGE,
                    raw_line=line,
                    file_path=match.group(1).strip(),
                    change_type="modified",
                )
            )

        # Check for Cursor file created pattern
        if match := self._CURSOR_FILE_CREATED.search(line):
            events.append(
                FileChangeEvent(
                    event_type=EventType.FILE_CHANGE,
                    raw_line=line,
                    file_path=match.group(1).strip(),
                    change_type="created",
                )
            )

        # Check for Cursor file deleted pattern
        if match := self._CURSOR_FILE_DELETED.search(line):
            events.append(
                FileChangeEvent(
                    event_type=EventType.FILE_CHANGE,
                    raw_line=line,
                    file_path=match.group(1).strip(),
                    change_type="deleted",
                )
            )

        # Check for error patterns
        # Traceback header
        if self._TRACEBACK_HEADER.search(line):
            events.append(
                ErrorEvent(
                    event_type=EventType.ERROR,
                    raw_line=line,
                    error_message="Traceback (most recent call last):",
                )
            )
        # Python exception types (e.g., ValueError:, TypeError:)
        elif match := self._PYTHON_EXCEPTION.search(line):
            events.append(
                ErrorEvent(
                    event_type=EventType.ERROR,
                    raw_line=line,
                    error_message=match.group(2).strip(),
                )
            )
        # Generic Error: prefix
        elif match := self._ERROR_PREFIX.search(line):
            events.append(
                ErrorEvent(
                    event_type=EventType.ERROR,
                    raw_line=line,
                    error_message=match.group(1).strip(),
                )
            )
        # Exception: prefix
        elif match := self._EXCEPTION_PREFIX.search(line):
            events.append(
                ErrorEvent(
                    event_type=EventType.ERROR,
                    raw_line=line,
                    error_message=match.group(1).strip(),
                )
            )

        # Check for warning patterns
        # Python warning types (e.g., DeprecationWarning:, UserWarning:)
        if match := self._PYTHON_WARNING.search(line):
            events.append(
                WarningEvent(
                    event_type=EventType.WARNING,
                    raw_line=line,
                    warning_message=match.group(2).strip(),
                )
            )
        # Generic Warning: prefix
        elif match := self._WARNING_PREFIX.search(line):
            events.append(
                WarningEvent(
                    event_type=EventType.WARNING,
                    raw_line=line,
                    warning_message=match.group(1).strip(),
                )
            )

        return events

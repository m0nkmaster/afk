"""Tests for output parser - event types and pattern detection."""

from __future__ import annotations

from afk.output_parser import (
    ErrorEvent,
    Event,
    EventType,
    FileChangeEvent,
    ToolCallEvent,
    WarningEvent,
)


class TestEventType:
    """Tests for the EventType enumeration."""

    def test_event_type_values(self) -> None:
        """Test EventType enum has expected values."""
        assert EventType.TOOL_CALL.value == "tool_call"
        assert EventType.FILE_CHANGE.value == "file_change"
        assert EventType.ERROR.value == "error"
        assert EventType.WARNING.value == "warning"

    def test_event_type_members(self) -> None:
        """Test EventType has all required members."""
        members = list(EventType)
        assert len(members) == 4
        assert EventType.TOOL_CALL in members
        assert EventType.FILE_CHANGE in members
        assert EventType.ERROR in members
        assert EventType.WARNING in members


class TestEvent:
    """Tests for the base Event dataclass."""

    def test_event_instantiation(self) -> None:
        """Test Event can be created with required fields."""
        event = Event(event_type=EventType.TOOL_CALL, raw_line="Test line")

        assert event.event_type == EventType.TOOL_CALL
        assert event.raw_line == "Test line"

    def test_event_with_different_types(self) -> None:
        """Test Event can be created with each event type."""
        for event_type in EventType:
            event = Event(event_type=event_type, raw_line=f"Line for {event_type}")
            assert event.event_type == event_type


class TestToolCallEvent:
    """Tests for the ToolCallEvent dataclass."""

    def test_tool_call_event_instantiation(self) -> None:
        """Test ToolCallEvent can be created with tool_name."""
        event = ToolCallEvent(
            event_type=EventType.TOOL_CALL,
            raw_line="Calling tool: write_file",
            tool_name="write_file",
        )

        assert event.event_type == EventType.TOOL_CALL
        assert event.raw_line == "Calling tool: write_file"
        assert event.tool_name == "write_file"

    def test_tool_call_event_various_tools(self) -> None:
        """Test ToolCallEvent with various tool names."""
        tools = ["read_file", "execute_command", "edit", "Write", "Shell"]
        for tool in tools:
            event = ToolCallEvent(
                event_type=EventType.TOOL_CALL,
                raw_line=f"Using {tool}",
                tool_name=tool,
            )
            assert event.tool_name == tool


class TestFileChangeEvent:
    """Tests for the FileChangeEvent dataclass."""

    def test_file_change_event_instantiation(self) -> None:
        """Test FileChangeEvent can be created with file_path and change_type."""
        event = FileChangeEvent(
            event_type=EventType.FILE_CHANGE,
            raw_line="Edited src/main.py",
            file_path="src/main.py",
            change_type="modified",
        )

        assert event.event_type == EventType.FILE_CHANGE
        assert event.raw_line == "Edited src/main.py"
        assert event.file_path == "src/main.py"
        assert event.change_type == "modified"

    def test_file_change_event_created(self) -> None:
        """Test FileChangeEvent for file creation."""
        event = FileChangeEvent(
            event_type=EventType.FILE_CHANGE,
            raw_line="Created new.py",
            file_path="new.py",
            change_type="created",
        )

        assert event.change_type == "created"
        assert event.file_path == "new.py"

    def test_file_change_event_deleted(self) -> None:
        """Test FileChangeEvent for file deletion."""
        event = FileChangeEvent(
            event_type=EventType.FILE_CHANGE,
            raw_line="Deleted old.py",
            file_path="old.py",
            change_type="deleted",
        )

        assert event.change_type == "deleted"
        assert event.file_path == "old.py"


class TestErrorEvent:
    """Tests for the ErrorEvent dataclass."""

    def test_error_event_instantiation(self) -> None:
        """Test ErrorEvent can be created with error_message."""
        event = ErrorEvent(
            event_type=EventType.ERROR,
            raw_line="Error: Something went wrong",
            error_message="Something went wrong",
        )

        assert event.event_type == EventType.ERROR
        assert event.raw_line == "Error: Something went wrong"
        assert event.error_message == "Something went wrong"

    def test_error_event_with_traceback(self) -> None:
        """Test ErrorEvent with a traceback message."""
        event = ErrorEvent(
            event_type=EventType.ERROR,
            raw_line="Traceback (most recent call last):",
            error_message="Traceback (most recent call last):",
        )

        assert event.error_message == "Traceback (most recent call last):"

    def test_error_event_with_exception(self) -> None:
        """Test ErrorEvent with an exception."""
        event = ErrorEvent(
            event_type=EventType.ERROR,
            raw_line="Exception: Connection refused",
            error_message="Connection refused",
        )

        assert event.error_message == "Connection refused"


class TestWarningEvent:
    """Tests for the WarningEvent dataclass."""

    def test_warning_event_instantiation(self) -> None:
        """Test WarningEvent can be created with warning_message."""
        event = WarningEvent(
            event_type=EventType.WARNING,
            raw_line="Warning: Deprecated function",
            warning_message="Deprecated function",
        )

        assert event.event_type == EventType.WARNING
        assert event.raw_line == "Warning: Deprecated function"
        assert event.warning_message == "Deprecated function"

    def test_warning_event_with_various_messages(self) -> None:
        """Test WarningEvent with various warning messages."""
        warnings = [
            "Unused import",
            "Variable may be undefined",
            "DeprecationWarning: Use new_func instead",
        ]
        for msg in warnings:
            event = WarningEvent(
                event_type=EventType.WARNING,
                raw_line=f"Warning: {msg}",
                warning_message=msg,
            )
            assert event.warning_message == msg


class TestEventInheritance:
    """Tests to verify event types share base Event properties."""

    def test_all_events_have_raw_line(self) -> None:
        """Test all event types include raw_line from base."""
        events = [
            ToolCallEvent(
                event_type=EventType.TOOL_CALL,
                raw_line="line1",
                tool_name="test",
            ),
            FileChangeEvent(
                event_type=EventType.FILE_CHANGE,
                raw_line="line2",
                file_path="test.py",
                change_type="modified",
            ),
            ErrorEvent(
                event_type=EventType.ERROR,
                raw_line="line3",
                error_message="error",
            ),
            WarningEvent(
                event_type=EventType.WARNING,
                raw_line="line4",
                warning_message="warning",
            ),
        ]

        for i, event in enumerate(events, start=1):
            assert event.raw_line == f"line{i}"

    def test_all_events_have_event_type(self) -> None:
        """Test all event types include event_type from base."""
        events = [
            (
                ToolCallEvent(
                    event_type=EventType.TOOL_CALL,
                    raw_line="",
                    tool_name="t",
                ),
                EventType.TOOL_CALL,
            ),
            (
                FileChangeEvent(
                    event_type=EventType.FILE_CHANGE,
                    raw_line="",
                    file_path="f",
                    change_type="c",
                ),
                EventType.FILE_CHANGE,
            ),
            (
                ErrorEvent(
                    event_type=EventType.ERROR,
                    raw_line="",
                    error_message="e",
                ),
                EventType.ERROR,
            ),
            (
                WarningEvent(
                    event_type=EventType.WARNING,
                    raw_line="",
                    warning_message="w",
                ),
                EventType.WARNING,
            ),
        ]

        for event, expected_type in events:
            assert event.event_type == expected_type


class TestOutputParser:
    """Tests for the OutputParser class."""

    def test_parser_instantiation(self) -> None:
        """Test OutputParser can be instantiated."""
        from afk.output_parser import OutputParser

        parser = OutputParser()
        assert parser is not None

    def test_parse_returns_list(self) -> None:
        """Test parse() returns a list."""
        from afk.output_parser import OutputParser

        parser = OutputParser()
        result = parser.parse("some line")
        assert isinstance(result, list)

    def test_parse_empty_line_returns_empty_list(self) -> None:
        """Test parse() returns empty list for empty input."""
        from afk.output_parser import OutputParser

        parser = OutputParser()
        result = parser.parse("")
        assert result == []

    def test_parse_non_matching_line_returns_empty_list(self) -> None:
        """Test parse() returns empty list for non-matching lines."""
        from afk.output_parser import OutputParser

        parser = OutputParser()
        result = parser.parse("Just some random text output")
        assert result == []


class TestOutputParserClaudeToolCalls:
    """Tests for Claude Code tool call pattern detection."""

    def test_detect_tool_call_write_file(self) -> None:
        """Test detection of write_file tool call."""
        from afk.output_parser import OutputParser

        parser = OutputParser()
        events = parser.parse("Calling tool: write_file")

        assert len(events) == 1
        assert isinstance(events[0], ToolCallEvent)
        assert events[0].event_type == EventType.TOOL_CALL
        assert events[0].tool_name == "write_file"
        assert events[0].raw_line == "Calling tool: write_file"

    def test_detect_tool_call_read_file(self) -> None:
        """Test detection of read_file tool call."""
        from afk.output_parser import OutputParser

        parser = OutputParser()
        events = parser.parse("Calling tool: read_file")

        assert len(events) == 1
        assert isinstance(events[0], ToolCallEvent)
        assert events[0].tool_name == "read_file"

    def test_detect_tool_call_execute_command(self) -> None:
        """Test detection of execute_command tool call."""
        from afk.output_parser import OutputParser

        parser = OutputParser()
        events = parser.parse("Calling tool: execute_command")

        assert len(events) == 1
        assert isinstance(events[0], ToolCallEvent)
        assert events[0].tool_name == "execute_command"

    def test_detect_various_tool_calls(self) -> None:
        """Test detection of various tool names."""
        from afk.output_parser import OutputParser

        parser = OutputParser()
        tools = ["edit", "search", "list_files", "bash"]

        for tool in tools:
            events = parser.parse(f"Calling tool: {tool}")
            assert len(events) == 1
            assert isinstance(events[0], ToolCallEvent)
            assert events[0].tool_name == tool

    def test_tool_call_case_sensitive(self) -> None:
        """Test tool call pattern is case sensitive for tool name."""
        from afk.output_parser import OutputParser

        parser = OutputParser()
        # Pattern should match the exact casing
        events = parser.parse("Calling tool: Write_File")
        assert len(events) == 1
        assert isinstance(events[0], ToolCallEvent)
        assert events[0].tool_name == "Write_File"

    def test_tool_call_with_prefix_text(self) -> None:
        """Test tool call detection even with prefix text."""
        from afk.output_parser import OutputParser

        parser = OutputParser()
        events = parser.parse("  [INFO] Calling tool: write_file")

        assert len(events) == 1
        assert isinstance(events[0], ToolCallEvent)
        assert events[0].tool_name == "write_file"


class TestOutputParserClaudeFileOperations:
    """Tests for Claude Code file operation pattern detection."""

    def test_detect_file_write(self) -> None:
        """Test detection of file write operation."""
        from afk.output_parser import OutputParser

        parser = OutputParser()
        events = parser.parse("Writing to: src/main.py")

        assert len(events) == 1
        assert isinstance(events[0], FileChangeEvent)
        assert events[0].event_type == EventType.FILE_CHANGE
        assert events[0].file_path == "src/main.py"
        assert events[0].change_type == "modified"

    def test_detect_file_write_with_spaces(self) -> None:
        """Test detection of file write with spaces in path."""
        from afk.output_parser import OutputParser

        parser = OutputParser()
        events = parser.parse("Writing to: src/my file.py")

        assert len(events) == 1
        assert isinstance(events[0], FileChangeEvent)
        assert events[0].file_path == "src/my file.py"

    def test_detect_file_write_deep_path(self) -> None:
        """Test detection of file write with deep directory path."""
        from afk.output_parser import OutputParser

        parser = OutputParser()
        events = parser.parse("Writing to: src/components/auth/LoginForm.tsx")

        assert len(events) == 1
        assert isinstance(events[0], FileChangeEvent)
        assert events[0].file_path == "src/components/auth/LoginForm.tsx"

    def test_detect_file_read(self) -> None:
        """Test detection of file read operation."""
        from afk.output_parser import OutputParser

        parser = OutputParser()
        events = parser.parse("Reading: config.json")

        assert len(events) == 1
        assert isinstance(events[0], FileChangeEvent)
        assert events[0].file_path == "config.json"
        assert events[0].change_type == "read"

    def test_detect_file_read_with_prefix(self) -> None:
        """Test detection of file read with prefix text."""
        from afk.output_parser import OutputParser

        parser = OutputParser()
        events = parser.parse("  Reading: tests/test_config.py")

        assert len(events) == 1
        assert isinstance(events[0], FileChangeEvent)
        assert events[0].file_path == "tests/test_config.py"

    def test_multiple_events_in_line(self) -> None:
        """Test that each line returns at most one event per pattern."""
        from afk.output_parser import OutputParser

        parser = OutputParser()
        # Line with tool call pattern
        events = parser.parse("Calling tool: write_file")
        assert len(events) == 1

    def test_file_write_absolute_path(self) -> None:
        """Test detection of file write with absolute path."""
        from afk.output_parser import OutputParser

        parser = OutputParser()
        events = parser.parse("Writing to: /home/user/project/file.py")

        assert len(events) == 1
        assert isinstance(events[0], FileChangeEvent)
        assert events[0].file_path == "/home/user/project/file.py"


class TestOutputParserReturnsCorrectTypes:
    """Tests ensuring parse() returns correct Event subtypes."""

    def test_tool_call_returns_tool_call_event(self) -> None:
        """Test tool call pattern returns ToolCallEvent."""
        from afk.output_parser import OutputParser

        parser = OutputParser()
        events = parser.parse("Calling tool: test_tool")

        assert len(events) == 1
        assert type(events[0]) is ToolCallEvent

    def test_file_write_returns_file_change_event(self) -> None:
        """Test file write pattern returns FileChangeEvent."""
        from afk.output_parser import OutputParser

        parser = OutputParser()
        events = parser.parse("Writing to: test.py")

        assert len(events) == 1
        assert type(events[0]) is FileChangeEvent

    def test_file_read_returns_file_change_event(self) -> None:
        """Test file read pattern returns FileChangeEvent."""
        from afk.output_parser import OutputParser

        parser = OutputParser()
        events = parser.parse("Reading: test.py")

        assert len(events) == 1
        assert type(events[0]) is FileChangeEvent

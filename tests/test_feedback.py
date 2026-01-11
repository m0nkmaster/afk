"""Tests for feedback module - iteration metrics and display."""

from __future__ import annotations

import json
from dataclasses import asdict
from datetime import datetime

from afk.feedback import IterationMetrics, MetricsCollector


class TestIterationMetrics:
    """Tests for the IterationMetrics dataclass."""

    def test_default_instantiation(self) -> None:
        """Test IterationMetrics can be created with defaults."""
        metrics = IterationMetrics()

        assert metrics.tool_calls == 0
        assert metrics.files_modified == []
        assert metrics.files_created == []
        assert metrics.files_deleted == []
        assert metrics.lines_added == 0
        assert metrics.lines_removed == 0
        assert metrics.errors == []
        assert metrics.warnings == []
        assert metrics.last_activity is None

    def test_instantiation_with_values(self) -> None:
        """Test IterationMetrics can be created with custom values."""
        now = datetime.now()
        metrics = IterationMetrics(
            tool_calls=5,
            files_modified=["src/main.py"],
            files_created=["src/new.py", "tests/test_new.py"],
            files_deleted=["old.py"],
            lines_added=100,
            lines_removed=50,
            errors=["SyntaxError: invalid syntax"],
            warnings=["Unused import"],
            last_activity=now,
        )

        assert metrics.tool_calls == 5
        assert metrics.files_modified == ["src/main.py"]
        assert metrics.files_created == ["src/new.py", "tests/test_new.py"]
        assert metrics.files_deleted == ["old.py"]
        assert metrics.lines_added == 100
        assert metrics.lines_removed == 50
        assert metrics.errors == ["SyntaxError: invalid syntax"]
        assert metrics.warnings == ["Unused import"]
        assert metrics.last_activity == now

    def test_serialisation_to_dict(self) -> None:
        """Test IterationMetrics can be serialised to dict via asdict."""
        metrics = IterationMetrics(
            tool_calls=3,
            files_modified=["a.py"],
            lines_added=10,
        )

        data = asdict(metrics)

        assert data["tool_calls"] == 3
        assert data["files_modified"] == ["a.py"]
        assert data["lines_added"] == 10
        assert data["files_created"] == []
        assert data["files_deleted"] == []

    def test_serialisation_to_json(self) -> None:
        """Test IterationMetrics can be serialised to JSON."""
        metrics = IterationMetrics(
            tool_calls=2,
            files_created=["new.py"],
        )

        # Convert to dict, handling datetime
        data = asdict(metrics)
        # last_activity is None by default, which is JSON-serialisable
        json_str = json.dumps(data, default=str)

        parsed = json.loads(json_str)
        assert parsed["tool_calls"] == 2
        assert parsed["files_created"] == ["new.py"]

    def test_list_fields_are_independent(self) -> None:
        """Test that list fields don't share state between instances."""
        metrics1 = IterationMetrics()
        metrics2 = IterationMetrics()

        metrics1.files_modified.append("a.py")

        assert metrics1.files_modified == ["a.py"]
        assert metrics2.files_modified == []

    def test_last_activity_with_datetime(self) -> None:
        """Test last_activity can store a datetime."""
        now = datetime.now()
        metrics = IterationMetrics(last_activity=now)

        assert metrics.last_activity == now
        assert isinstance(metrics.last_activity, datetime)


class TestMetricsCollector:
    """Tests for the MetricsCollector class."""

    def test_default_instantiation(self) -> None:
        """Test MetricsCollector can be created with default empty metrics."""
        collector = MetricsCollector()

        assert collector.metrics.tool_calls == 0
        assert collector.metrics.files_modified == []
        assert collector.metrics.files_created == []
        assert collector.metrics.files_deleted == []

    def test_record_tool_call_increments_count(self) -> None:
        """Test record_tool_call increments the tool_calls counter."""
        collector = MetricsCollector()

        collector.record_tool_call("write_file")
        assert collector.metrics.tool_calls == 1

        collector.record_tool_call("read_file")
        assert collector.metrics.tool_calls == 2

        collector.record_tool_call("execute_command")
        assert collector.metrics.tool_calls == 3

    def test_record_file_change_modified(self) -> None:
        """Test record_file_change adds modified files to the list."""
        collector = MetricsCollector()

        collector.record_file_change("src/main.py", "modified")

        assert "src/main.py" in collector.metrics.files_modified
        assert len(collector.metrics.files_modified) == 1
        assert collector.metrics.files_created == []
        assert collector.metrics.files_deleted == []

    def test_record_file_change_created(self) -> None:
        """Test record_file_change adds created files to the list."""
        collector = MetricsCollector()

        collector.record_file_change("src/new.py", "created")

        assert "src/new.py" in collector.metrics.files_created
        assert len(collector.metrics.files_created) == 1
        assert collector.metrics.files_modified == []
        assert collector.metrics.files_deleted == []

    def test_record_file_change_deleted(self) -> None:
        """Test record_file_change adds deleted files to the list."""
        collector = MetricsCollector()

        collector.record_file_change("old.py", "deleted")

        assert "old.py" in collector.metrics.files_deleted
        assert len(collector.metrics.files_deleted) == 1
        assert collector.metrics.files_modified == []
        assert collector.metrics.files_created == []

    def test_record_file_change_multiple_types(self) -> None:
        """Test recording multiple file changes of different types."""
        collector = MetricsCollector()

        collector.record_file_change("modified.py", "modified")
        collector.record_file_change("created.py", "created")
        collector.record_file_change("deleted.py", "deleted")
        collector.record_file_change("also_modified.py", "modified")

        assert collector.metrics.files_modified == ["modified.py", "also_modified.py"]
        assert collector.metrics.files_created == ["created.py"]
        assert collector.metrics.files_deleted == ["deleted.py"]

    def test_record_file_change_updates_last_activity(self) -> None:
        """Test record_file_change updates last_activity timestamp."""
        collector = MetricsCollector()
        assert collector.metrics.last_activity is None

        collector.record_file_change("test.py", "modified")

        assert collector.metrics.last_activity is not None
        assert isinstance(collector.metrics.last_activity, datetime)

    def test_record_tool_call_updates_last_activity(self) -> None:
        """Test record_tool_call updates last_activity timestamp."""
        collector = MetricsCollector()
        assert collector.metrics.last_activity is None

        collector.record_tool_call("some_tool")

        assert collector.metrics.last_activity is not None
        assert isinstance(collector.metrics.last_activity, datetime)

    def test_reset_clears_all_metrics(self) -> None:
        """Test reset() clears all accumulated metrics."""
        collector = MetricsCollector()

        # Accumulate some metrics
        collector.record_tool_call("tool1")
        collector.record_tool_call("tool2")
        collector.record_file_change("a.py", "modified")
        collector.record_file_change("b.py", "created")
        collector.record_file_change("c.py", "deleted")

        # Verify we have data
        assert collector.metrics.tool_calls == 2
        assert len(collector.metrics.files_modified) == 1
        assert len(collector.metrics.files_created) == 1
        assert len(collector.metrics.files_deleted) == 1
        assert collector.metrics.last_activity is not None

        # Reset
        collector.reset()

        # Verify everything is cleared
        assert collector.metrics.tool_calls == 0
        assert collector.metrics.files_modified == []
        assert collector.metrics.files_created == []
        assert collector.metrics.files_deleted == []
        assert collector.metrics.lines_added == 0
        assert collector.metrics.lines_removed == 0
        assert collector.metrics.errors == []
        assert collector.metrics.warnings == []
        assert collector.metrics.last_activity is None

    def test_reset_creates_new_metrics_instance(self) -> None:
        """Test reset() creates a fresh IterationMetrics instance."""
        collector = MetricsCollector()
        collector.record_tool_call("tool")
        old_metrics = collector.metrics

        collector.reset()

        # Should be a different instance
        assert collector.metrics is not old_metrics

    def test_metrics_property_returns_current_metrics(self) -> None:
        """Test metrics property provides access to current IterationMetrics."""
        collector = MetricsCollector()
        collector.record_tool_call("test")

        metrics = collector.metrics

        assert isinstance(metrics, IterationMetrics)
        assert metrics.tool_calls == 1

    def test_record_file_change_ignores_unknown_change_type(self) -> None:
        """Test record_file_change handles unknown change types gracefully."""
        collector = MetricsCollector()

        # Should not raise, but also should not add to any list
        collector.record_file_change("unknown.py", "renamed")

        assert collector.metrics.files_modified == []
        assert collector.metrics.files_created == []
        assert collector.metrics.files_deleted == []
        # But last_activity should still be updated
        assert collector.metrics.last_activity is not None


class TestFeedbackDisplay:
    """Tests for the FeedbackDisplay class."""

    def test_instantiation(self) -> None:
        """Test FeedbackDisplay can be created."""
        from afk.feedback import FeedbackDisplay

        display = FeedbackDisplay()

        assert display is not None
        assert display._live is None  # Not started yet

    def test_start_initialises_live_context(self) -> None:
        """Test start() initialises Rich Live context."""
        from afk.feedback import FeedbackDisplay

        display = FeedbackDisplay()
        display.start()

        try:
            assert display._live is not None
            assert display._started is True
        finally:
            display.stop()

    def test_stop_exits_live_context(self) -> None:
        """Test stop() cleanly exits Live context."""
        from afk.feedback import FeedbackDisplay

        display = FeedbackDisplay()
        display.start()
        display.stop()

        # After stop, started flag should be False
        assert display._started is False

    def test_stop_before_start_is_safe(self) -> None:
        """Test stop() is safe to call without start()."""
        from afk.feedback import FeedbackDisplay

        display = FeedbackDisplay()
        # Should not raise
        display.stop()

    def test_build_panel_returns_renderable(self) -> None:
        """Test _build_panel returns a Rich Panel."""
        from rich.console import Console
        from rich.panel import Panel

        from afk.feedback import FeedbackDisplay

        display = FeedbackDisplay()
        panel = display._build_panel()

        # Verify it's renderable by using Console
        console = Console(force_terminal=True, width=80)
        # If it's not renderable, this would raise
        with console.capture():
            console.print(panel)

        assert isinstance(panel, Panel)

    def test_build_panel_contains_header(self) -> None:
        """Test _build_panel includes header with afk title."""
        from rich.console import Console

        from afk.feedback import FeedbackDisplay

        display = FeedbackDisplay()
        panel = display._build_panel()

        console = Console(force_terminal=True, width=80)
        with console.capture() as capture:
            console.print(panel)

        output = capture.get()
        assert "afk" in output.lower()

    def test_double_start_is_safe(self) -> None:
        """Test calling start() twice doesn't crash."""
        from afk.feedback import FeedbackDisplay

        display = FeedbackDisplay()
        display.start()

        try:
            # Second start should be a no-op or safe
            display.start()
            assert display._live is not None
        finally:
            display.stop()

    def test_double_stop_is_safe(self) -> None:
        """Test calling stop() twice doesn't crash."""
        from afk.feedback import FeedbackDisplay

        display = FeedbackDisplay()
        display.start()
        display.stop()
        # Second stop should be safe
        display.stop()

    def test_build_activity_panel_returns_panel(self) -> None:
        """Test _build_activity_panel returns a Rich Panel."""
        from rich.panel import Panel

        from afk.feedback import FeedbackDisplay

        display = FeedbackDisplay()
        metrics = IterationMetrics(
            tool_calls=3,
            files_modified=["a.py"],
            files_created=["b.py"],
            lines_added=50,
            lines_removed=10,
        )

        panel = display._build_activity_panel(metrics)

        assert isinstance(panel, Panel)

    def test_build_activity_panel_shows_spinner(self) -> None:
        """Test _build_activity_panel includes spinner character."""
        from rich.console import Console

        from afk.feedback import FeedbackDisplay

        display = FeedbackDisplay()
        metrics = IterationMetrics()

        panel = display._build_activity_panel(metrics)

        console = Console(force_terminal=True, width=80)
        with console.capture() as capture:
            console.print(panel)

        output = capture.get()
        # The dots spinner includes characters like ⠋, ⠙, etc.
        # At frame 0, should be ⠋
        assert "⠋" in output

    def test_build_activity_panel_shows_tool_count(self) -> None:
        """Test _build_activity_panel displays tool call count."""
        from rich.console import Console

        from afk.feedback import FeedbackDisplay

        display = FeedbackDisplay()
        metrics = IterationMetrics(tool_calls=5)

        panel = display._build_activity_panel(metrics)

        console = Console(force_terminal=True, width=80)
        with console.capture() as capture:
            console.print(panel)

        output = capture.get()
        assert "Tools" in output
        assert "5" in output

    def test_build_activity_panel_shows_files_touched(self) -> None:
        """Test _build_activity_panel displays files touched count."""
        from rich.console import Console

        from afk.feedback import FeedbackDisplay

        display = FeedbackDisplay()
        metrics = IterationMetrics(
            files_modified=["a.py", "b.py"],
            files_created=["c.py"],
            files_deleted=["d.py"],
        )

        panel = display._build_activity_panel(metrics)

        console = Console(force_terminal=True, width=80)
        with console.capture() as capture:
            console.print(panel)

        output = capture.get()
        assert "Files" in output
        # 2 modified + 1 created + 1 deleted = 4
        assert "4" in output

    def test_build_activity_panel_shows_line_changes(self) -> None:
        """Test _build_activity_panel displays lines added/removed."""
        from rich.console import Console

        from afk.feedback import FeedbackDisplay

        display = FeedbackDisplay()
        metrics = IterationMetrics(
            lines_added=100,
            lines_removed=25,
        )

        panel = display._build_activity_panel(metrics)

        console = Console(force_terminal=True, width=80)
        with console.capture() as capture:
            console.print(panel)

        output = capture.get()
        assert "Lines" in output
        assert "+100" in output
        assert "-25" in output

    def test_update_increments_spinner_frame(self) -> None:
        """Test update() increments the spinner frame index."""
        from afk.feedback import FeedbackDisplay

        display = FeedbackDisplay()
        display.start()

        try:
            assert display._spinner_frame == 0

            metrics = IterationMetrics()
            display.update(metrics)

            assert display._spinner_frame == 1

            display.update(metrics)
            display.update(metrics)

            assert display._spinner_frame == 3
        finally:
            display.stop()

    def test_update_without_start_is_safe(self) -> None:
        """Test update() is safe to call without start()."""
        from afk.feedback import FeedbackDisplay

        display = FeedbackDisplay()
        metrics = IterationMetrics()

        # Should not raise
        display.update(metrics)

        # Spinner frame should not increment when not started
        assert display._spinner_frame == 0

    def test_build_panel_with_metrics(self) -> None:
        """Test _build_panel includes activity panel when metrics provided."""
        from rich.console import Console

        from afk.feedback import FeedbackDisplay

        display = FeedbackDisplay()
        metrics = IterationMetrics(
            tool_calls=2,
            files_modified=["test.py"],
        )

        panel = display._build_panel(metrics)

        console = Console(force_terminal=True, width=80)
        with console.capture() as capture:
            console.print(panel)

        output = capture.get()
        # Should include activity info, not "Waiting for activity"
        assert "Activity" in output or "Tools" in output
        assert "Waiting for activity" not in output

    def test_build_files_panel_returns_panel(self) -> None:
        """Test _build_files_panel returns a Rich Panel."""
        from rich.panel import Panel

        from afk.feedback import FeedbackDisplay

        display = FeedbackDisplay()
        metrics = IterationMetrics(
            files_modified=["src/main.py"],
            files_created=["src/new.py"],
        )

        panel = display._build_files_panel(metrics)

        assert isinstance(panel, Panel)

    def test_build_files_panel_shows_modified_with_pencil(self) -> None:
        """Test _build_files_panel shows modified files with ✎ prefix."""
        from rich.console import Console

        from afk.feedback import FeedbackDisplay

        display = FeedbackDisplay()
        metrics = IterationMetrics(
            files_modified=["src/main.py"],
        )

        panel = display._build_files_panel(metrics)

        console = Console(force_terminal=True, width=80)
        with console.capture() as capture:
            console.print(panel)

        output = capture.get()
        assert "✎" in output
        assert "main.py" in output

    def test_build_files_panel_shows_created_with_plus(self) -> None:
        """Test _build_files_panel shows created files with + prefix."""
        from rich.console import Console

        from afk.feedback import FeedbackDisplay

        display = FeedbackDisplay()
        metrics = IterationMetrics(
            files_created=["src/new.py"],
        )

        panel = display._build_files_panel(metrics)

        console = Console(force_terminal=True, width=80)
        with console.capture() as capture:
            console.print(panel)

        output = capture.get()
        assert "+" in output
        assert "new.py" in output

    def test_build_files_panel_limits_to_five_files(self) -> None:
        """Test _build_files_panel shows at most 5 files."""
        from rich.console import Console

        from afk.feedback import FeedbackDisplay

        display = FeedbackDisplay()
        metrics = IterationMetrics(
            files_modified=["a.py", "b.py", "c.py", "d.py", "e.py", "f.py", "g.py"],
        )

        panel = display._build_files_panel(metrics)

        console = Console(force_terminal=True, width=80)
        with console.capture() as capture:
            console.print(panel)

        output = capture.get()
        # The last 5 files (most recent) should be shown: c.py, d.py, e.py, f.py, g.py
        # First 2 (a.py, b.py) should not appear
        assert "g.py" in output
        assert "f.py" in output
        assert "a.py" not in output

    def test_build_files_panel_truncates_long_paths(self) -> None:
        """Test _build_files_panel truncates paths that are too long."""
        from rich.console import Console

        from afk.feedback import FeedbackDisplay

        display = FeedbackDisplay()
        long_path = "src/very/deeply/nested/folder/structure/with/many/levels/file.py"
        metrics = IterationMetrics(
            files_modified=[long_path],
        )

        panel = display._build_files_panel(metrics)

        console = Console(force_terminal=True, width=60)
        with console.capture() as capture:
            console.print(panel)

        output = capture.get()
        # Should contain truncation indicator or the filename
        assert "file.py" in output or "..." in output

    def test_build_files_panel_empty_when_no_files(self) -> None:
        """Test _build_files_panel handles empty file lists."""
        from rich.panel import Panel

        from afk.feedback import FeedbackDisplay

        display = FeedbackDisplay()
        metrics = IterationMetrics()

        panel = display._build_files_panel(metrics)

        assert isinstance(panel, Panel)

    def test_build_files_panel_combines_modified_and_created(self) -> None:
        """Test _build_files_panel shows both modified and created files."""
        from rich.console import Console

        from afk.feedback import FeedbackDisplay

        display = FeedbackDisplay()
        metrics = IterationMetrics(
            files_modified=["existing.py"],
            files_created=["new.py"],
        )

        panel = display._build_files_panel(metrics)

        console = Console(force_terminal=True, width=80)
        with console.capture() as capture:
            console.print(panel)

        output = capture.get()
        assert "✎" in output
        assert "+" in output
        assert "existing.py" in output
        assert "new.py" in output

    def test_update_with_changing_metrics(self) -> None:
        """Test update() correctly refreshes display with changing metrics."""
        from afk.feedback import FeedbackDisplay

        display = FeedbackDisplay()
        display.start()

        try:
            # Initial update with baseline metrics
            metrics1 = IterationMetrics(
                tool_calls=1,
                files_modified=["first.py"],
            )
            display.update(metrics1)

            assert display._spinner_frame == 1

            # Update with changed metrics
            metrics2 = IterationMetrics(
                tool_calls=5,
                files_modified=["first.py", "second.py"],
                files_created=["new.py"],
                lines_added=100,
            )
            display.update(metrics2)

            assert display._spinner_frame == 2

            # The Live context should have been updated
            # We verify by checking the internal state was processed
            assert display._live is not None
        finally:
            display.stop()

    def test_update_rebuilds_panel_each_call(self) -> None:
        """Test update() rebuilds the panel on each call with current metrics."""
        from rich.console import Console

        from afk.feedback import FeedbackDisplay

        display = FeedbackDisplay()

        # Build panel with different metrics to verify rebuilding
        metrics1 = IterationMetrics(tool_calls=1)
        metrics2 = IterationMetrics(tool_calls=10)

        console = Console(force_terminal=True, width=80)

        with console.capture() as capture1:
            panel1 = display._build_panel(metrics1)
            console.print(panel1)
        output1 = capture1.get()

        with console.capture() as capture2:
            panel2 = display._build_panel(metrics2)
            console.print(panel2)
        output2 = capture2.get()

        # Tool counts should be different in each output
        assert "1" in output1
        assert "10" in output2

    def test_update_with_iteration_info(self) -> None:
        """Test update() accepts iteration_current and iteration_total."""
        from afk.feedback import FeedbackDisplay

        display = FeedbackDisplay()
        display.start()

        try:
            metrics = IterationMetrics()
            display.update(metrics, iteration_current=3, iteration_total=10)

            # Should store the iteration info for header display
            assert display._iteration_current == 3
            assert display._iteration_total == 10
        finally:
            display.stop()

    def test_build_panel_shows_iteration_count(self) -> None:
        """Test _build_panel header includes iteration x/y format."""
        from rich.console import Console

        from afk.feedback import FeedbackDisplay

        display = FeedbackDisplay()
        display._iteration_current = 5
        display._iteration_total = 20
        metrics = IterationMetrics()

        panel = display._build_panel(metrics)

        console = Console(force_terminal=True, width=80)
        with console.capture() as capture:
            console.print(panel)

        output = capture.get()
        # Should show "Iteration 5/20" or similar format
        assert "5/20" in output or ("5" in output and "20" in output)

    def test_start_time_tracking(self) -> None:
        """Test start() sets start_time for elapsed time calculation."""
        from afk.feedback import FeedbackDisplay

        display = FeedbackDisplay()
        assert display._start_time is None

        display.start()

        try:
            assert display._start_time is not None
            assert isinstance(display._start_time, datetime)
        finally:
            display.stop()

    def test_build_panel_shows_elapsed_time(self) -> None:
        """Test _build_panel header includes elapsed time in mm:ss format."""
        from rich.console import Console

        from afk.feedback import FeedbackDisplay

        display = FeedbackDisplay()
        # Manually set start_time for predictable test
        display._start_time = datetime.now()
        display._iteration_current = 1
        display._iteration_total = 5
        metrics = IterationMetrics()

        panel = display._build_panel(metrics)

        console = Console(force_terminal=True, width=80)
        with console.capture() as capture:
            console.print(panel)

        output = capture.get()
        # Should contain time in format like "00:00" or "0:00"
        assert ":" in output  # Time separator should be present

    def test_header_format(self) -> None:
        """Test header follows expected format: ◉ afk running ... Iteration x/y mm:ss."""
        from rich.console import Console

        from afk.feedback import FeedbackDisplay

        display = FeedbackDisplay()
        display._start_time = datetime.now()
        display._iteration_current = 2
        display._iteration_total = 8
        metrics = IterationMetrics()

        panel = display._build_panel(metrics)

        console = Console(force_terminal=True, width=100)
        with console.capture() as capture:
            console.print(panel)

        output = capture.get()
        # Should contain key elements
        assert "afk" in output.lower()
        assert "2/8" in output or "2 / 8" in output

    def test_update_without_iteration_params_uses_defaults(self) -> None:
        """Test update() works without iteration parameters using defaults."""
        from afk.feedback import FeedbackDisplay

        display = FeedbackDisplay()
        display.start()

        try:
            metrics = IterationMetrics()
            # Call update without iteration params - should not crash
            display.update(metrics)

            # Defaults should be 0/0 or similar
            assert display._iteration_current == 0
            assert display._iteration_total == 0
        finally:
            display.stop()

    def test_minimal_mode_instantiation(self) -> None:
        """Test FeedbackDisplay can be created with minimal mode."""
        from afk.feedback import FeedbackDisplay

        display = FeedbackDisplay(mode="minimal")

        assert display._mode == "minimal"

    def test_default_mode_is_full(self) -> None:
        """Test FeedbackDisplay defaults to full mode."""
        from afk.feedback import FeedbackDisplay

        display = FeedbackDisplay()

        assert display._mode == "full"

    def test_build_minimal_bar_returns_text(self) -> None:
        """Test _build_minimal_bar returns a Rich Text object."""
        from rich.text import Text

        from afk.feedback import FeedbackDisplay

        display = FeedbackDisplay(mode="minimal")
        metrics = IterationMetrics()

        bar = display._build_minimal_bar(metrics)

        assert isinstance(bar, Text)

    def test_build_minimal_bar_contains_afk_indicator(self) -> None:
        """Test _build_minimal_bar includes ◉ afk prefix."""
        from rich.console import Console

        from afk.feedback import FeedbackDisplay

        display = FeedbackDisplay(mode="minimal")
        metrics = IterationMetrics()

        bar = display._build_minimal_bar(metrics)

        console = Console(force_terminal=True, width=80)
        with console.capture() as capture:
            console.print(bar)

        output = capture.get()
        assert "◉" in output
        assert "afk" in output

    def test_build_minimal_bar_shows_iteration_count(self) -> None:
        """Test _build_minimal_bar includes iteration [x/y] format."""
        from rich.console import Console

        from afk.feedback import FeedbackDisplay

        display = FeedbackDisplay(mode="minimal")
        display._iteration_current = 5
        display._iteration_total = 20
        metrics = IterationMetrics()

        bar = display._build_minimal_bar(metrics)

        console = Console(force_terminal=True, width=80)
        with console.capture() as capture:
            console.print(bar)

        output = capture.get()
        assert "[5/20]" in output

    def test_build_minimal_bar_shows_elapsed_time(self) -> None:
        """Test _build_minimal_bar includes elapsed time mm:ss."""
        from rich.console import Console

        from afk.feedback import FeedbackDisplay

        display = FeedbackDisplay(mode="minimal")
        display._start_time = datetime.now()
        metrics = IterationMetrics()

        bar = display._build_minimal_bar(metrics)

        console = Console(force_terminal=True, width=80)
        with console.capture() as capture:
            console.print(bar)

        output = capture.get()
        # Should contain time format like "00:00"
        assert ":" in output

    def test_build_minimal_bar_shows_spinner(self) -> None:
        """Test _build_minimal_bar includes spinner character."""
        from rich.console import Console

        from afk.feedback import FeedbackDisplay

        display = FeedbackDisplay(mode="minimal")
        metrics = IterationMetrics()

        bar = display._build_minimal_bar(metrics)

        console = Console(force_terminal=True, width=80)
        with console.capture() as capture:
            console.print(bar)

        output = capture.get()
        # Should contain a spinner character from dots sequence
        dots_chars = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]
        assert any(char in output for char in dots_chars)

    def test_build_minimal_bar_shows_tool_calls(self) -> None:
        """Test _build_minimal_bar shows N calls."""
        from rich.console import Console

        from afk.feedback import FeedbackDisplay

        display = FeedbackDisplay(mode="minimal")
        metrics = IterationMetrics(tool_calls=7)

        bar = display._build_minimal_bar(metrics)

        console = Console(force_terminal=True, width=80)
        with console.capture() as capture:
            console.print(bar)

        output = capture.get()
        assert "7" in output
        assert "calls" in output.lower()

    def test_build_minimal_bar_shows_files_count(self) -> None:
        """Test _build_minimal_bar shows N files."""
        from rich.console import Console

        from afk.feedback import FeedbackDisplay

        display = FeedbackDisplay(mode="minimal")
        metrics = IterationMetrics(
            files_modified=["a.py", "b.py"],
            files_created=["c.py"],
        )

        bar = display._build_minimal_bar(metrics)

        console = Console(force_terminal=True, width=80)
        with console.capture() as capture:
            console.print(bar)

        output = capture.get()
        assert "3" in output
        assert "files" in output.lower()

    def test_build_minimal_bar_shows_line_changes(self) -> None:
        """Test _build_minimal_bar shows +N/-N line changes."""
        from rich.console import Console

        from afk.feedback import FeedbackDisplay

        display = FeedbackDisplay(mode="minimal")
        metrics = IterationMetrics(
            lines_added=50,
            lines_removed=12,
        )

        bar = display._build_minimal_bar(metrics)

        console = Console(force_terminal=True, width=80)
        with console.capture() as capture:
            console.print(bar)

        output = capture.get()
        assert "+50" in output
        assert "-12" in output

    def test_build_minimal_bar_uses_separator(self) -> None:
        """Test _build_minimal_bar uses │ as section separator."""
        from rich.console import Console

        from afk.feedback import FeedbackDisplay

        display = FeedbackDisplay(mode="minimal")
        metrics = IterationMetrics(tool_calls=1)

        bar = display._build_minimal_bar(metrics)

        console = Console(force_terminal=True, width=80)
        with console.capture() as capture:
            console.print(bar)

        output = capture.get()
        assert "│" in output

    def test_minimal_mode_uses_minimal_bar_in_update(self) -> None:
        """Test update() uses minimal bar when mode='minimal'."""
        from afk.feedback import FeedbackDisplay

        display = FeedbackDisplay(mode="minimal")
        display.start()

        try:
            metrics = IterationMetrics(tool_calls=3)
            display.update(metrics)

            # In minimal mode, the Live display should have been updated
            # with something (we can't easily inspect Rich's internal state,
            # but we verify it doesn't crash and the display is running)
            assert display._started is True
            assert display._live is not None
        finally:
            display.stop()

    def test_minimal_mode_renders_single_line(self) -> None:
        """Test minimal mode renders as a single line (no panels)."""
        from rich.console import Console

        from afk.feedback import FeedbackDisplay

        display = FeedbackDisplay(mode="minimal")
        display._start_time = datetime.now()
        display._iteration_current = 1
        display._iteration_total = 5
        metrics = IterationMetrics(
            tool_calls=2,
            files_modified=["test.py"],
            lines_added=10,
            lines_removed=5,
        )

        bar = display._build_minimal_bar(metrics)

        console = Console(force_terminal=True, width=100)
        with console.capture() as capture:
            console.print(bar)

        output = capture.get()
        # Single line output should not have Rich panel borders
        assert "╭" not in output  # No panel top border
        assert "╰" not in output  # No panel bottom border

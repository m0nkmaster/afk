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

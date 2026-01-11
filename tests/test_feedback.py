"""Tests for feedback module - iteration metrics and display."""

from __future__ import annotations

import json
from dataclasses import asdict
from datetime import datetime

from afk.feedback import IterationMetrics


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

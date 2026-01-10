"""Tests for afk.sources module."""

from __future__ import annotations

from afk.config import SourceConfig
from afk.sources import Task, _load_from_source, aggregate_tasks


class TestTask:
    """Tests for Task dataclass."""

    def test_defaults(self) -> None:
        """Test default values."""
        task = Task(id="task-1", description="Do something")
        assert task.id == "task-1"
        assert task.description == "Do something"
        assert task.priority == "medium"
        assert task.source == "unknown"
        assert task.metadata is None

    def test_all_fields(self) -> None:
        """Test all fields populated."""
        task = Task(
            id="task-2",
            description="Another task",
            priority="high",
            source="beads",
            metadata={"key": "value"},
        )
        assert task.priority == "high"
        assert task.source == "beads"
        assert task.metadata == {"key": "value"}


class TestAggregateTasksIntegration:
    """Integration tests for aggregate_tasks function."""

    def test_empty_sources(self) -> None:
        """Test with no sources."""
        tasks = aggregate_tasks([])
        assert tasks == []

    def test_multiple_sources(self, sample_tasks_json: None, sample_tasks_md: None) -> None:
        """Test aggregating from multiple sources."""
        sources = [
            SourceConfig(type="json", path="tasks.json"),
            SourceConfig(type="markdown", path="tasks.md"),
        ]
        tasks = aggregate_tasks(sources)
        # Should have tasks from both sources (excluding completed ones)
        assert len(tasks) >= 2


class TestLoadFromSource:
    """Tests for _load_from_source function."""

    def test_beads_source_without_bd(self) -> None:
        """Test beads source when bd is not installed."""
        from unittest.mock import patch

        source = SourceConfig(type="beads")
        with patch("subprocess.run") as mock_run:
            mock_run.side_effect = FileNotFoundError()
            tasks = _load_from_source(source)
            assert tasks == []

    def test_json_source(self, sample_tasks_json: None) -> None:
        """Test loading from JSON source."""
        source = SourceConfig(type="json", path="tasks.json")
        tasks = _load_from_source(source)
        assert len(tasks) == 2  # One is marked as passes=True
        assert tasks[0].id == "task-1"
        assert tasks[0].source == "json:tasks.json"

    def test_markdown_source(self, sample_tasks_md: None) -> None:
        """Test loading from markdown source."""
        source = SourceConfig(type="markdown", path="tasks.md")
        tasks = _load_from_source(source)
        assert len(tasks) == 3  # One is checked off
        assert any(t.id == "task-1" for t in tasks)

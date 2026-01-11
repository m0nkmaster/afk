"""Tests for afk.sources module."""

from __future__ import annotations

from afk.config import SourceConfig
from afk.prd_store import UserStory
from afk.sources import _load_from_source, aggregate_tasks


class TestUserStoryFromSources:
    """Tests for UserStory objects returned from sources."""

    def test_defaults(self) -> None:
        """Test default values."""
        story = UserStory(id="task-1", title="Do something", description="Details")
        assert story.id == "task-1"
        assert story.title == "Do something"
        assert story.description == "Details"
        assert story.priority == 3
        assert story.source == "unknown"
        assert story.passes is False
        assert story.acceptance_criteria == []

    def test_all_fields(self) -> None:
        """Test all fields populated."""
        story = UserStory(
            id="task-2",
            title="Another task",
            description="Full description",
            acceptance_criteria=["Step 1", "Step 2"],
            priority=1,
            passes=False,
            source="beads",
            notes="Some notes",
        )
        assert story.priority == 1
        assert story.source == "beads"
        assert len(story.acceptance_criteria) == 2


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
        # All returned tasks should be UserStory instances
        for task in tasks:
            assert isinstance(task, UserStory)


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
        assert isinstance(tasks[0], UserStory)

    def test_markdown_source(self, sample_tasks_md: None) -> None:
        """Test loading from markdown source."""
        source = SourceConfig(type="markdown", path="tasks.md")
        tasks = _load_from_source(source)
        assert len(tasks) == 3  # One is checked off
        assert any(t.id == "task-1" for t in tasks)
        assert all(isinstance(t, UserStory) for t in tasks)

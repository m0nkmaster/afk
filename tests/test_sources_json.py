"""Tests for afk.sources.json_prd module."""

from __future__ import annotations

import json
from pathlib import Path

from afk.sources.json_prd import (
    _generate_id,
    _map_priority,
    load_json_tasks,
)


class TestMapPriority:
    """Tests for _map_priority function."""

    def test_none_priority(self) -> None:
        """Test None returns medium."""
        assert _map_priority(None) == "medium"

    def test_integer_high(self) -> None:
        """Test low integers map to high."""
        assert _map_priority(0) == "high"
        assert _map_priority(1) == "high"

    def test_integer_medium(self) -> None:
        """Test mid integer maps to medium."""
        assert _map_priority(2) == "medium"

    def test_integer_low(self) -> None:
        """Test high integers map to low."""
        assert _map_priority(3) == "low"
        assert _map_priority(5) == "low"

    def test_string_high(self) -> None:
        """Test high priority strings."""
        assert _map_priority("high") == "high"
        assert _map_priority("critical") == "high"
        assert _map_priority("urgent") == "high"
        assert _map_priority("1") == "high"

    def test_string_low(self) -> None:
        """Test low priority strings."""
        assert _map_priority("low") == "low"
        assert _map_priority("minor") == "low"
        assert _map_priority("3") == "low"
        assert _map_priority("5") == "low"

    def test_string_medium(self) -> None:
        """Test medium priority strings."""
        assert _map_priority("medium") == "medium"
        assert _map_priority("normal") == "medium"
        assert _map_priority("2") == "medium"


class TestGenerateId:
    """Tests for _generate_id function."""

    def test_simple_description(self) -> None:
        """Test simple description."""
        assert _generate_id("Add new feature") == "add-new-feature"

    def test_long_description(self) -> None:
        """Test truncation of long descriptions."""
        long_desc = "This is a very long description that should be truncated to thirty chars"
        result = _generate_id(long_desc)
        assert len(result) <= 35  # 30 chars + some dashes

    def test_special_characters(self) -> None:
        """Test special characters are removed."""
        assert _generate_id("Fix bug #123!") == "fix-bug-123"

    def test_empty_description(self) -> None:
        """Test empty description."""
        assert _generate_id("") == "task"

    def test_only_special_chars(self) -> None:
        """Test description with only special chars."""
        assert _generate_id("!@#$%") == "task"


class TestLoadJsonTasks:
    """Tests for load_json_tasks function."""

    def test_no_path_no_default_files(self, temp_project: Path) -> None:
        """Test with no path and no default files."""
        tasks = load_json_tasks(None)
        assert tasks == []

    def test_no_path_finds_default(self, temp_project: Path) -> None:
        """Test finding default prd.json."""
        data = [{"id": "task-1", "description": "Default task"}]
        (temp_project / "prd.json").write_text(json.dumps(data))

        tasks = load_json_tasks(None)
        assert len(tasks) == 1
        assert tasks[0].id == "task-1"

    def test_no_path_finds_tasks_json(self, temp_project: Path) -> None:
        """Test finding default tasks.json."""
        data = [{"id": "task-1", "description": "Task from tasks.json"}]
        (temp_project / "tasks.json").write_text(json.dumps(data))

        tasks = load_json_tasks(None)
        assert len(tasks) == 1

    def test_file_not_found(self, temp_project: Path) -> None:
        """Test with non-existent file."""
        tasks = load_json_tasks("nonexistent.json")
        assert tasks == []

    def test_array_format(self, temp_project: Path) -> None:
        """Test loading array format."""
        data = [
            {"id": "task-1", "description": "First"},
            {"id": "task-2", "description": "Second"},
        ]
        path = temp_project / "tasks.json"
        path.write_text(json.dumps(data))

        tasks = load_json_tasks(str(path))
        assert len(tasks) == 2
        assert tasks[0].id == "task-1"
        assert tasks[0].source == f"json:{path}"

    def test_object_with_tasks_key(self, temp_project: Path) -> None:
        """Test loading object with 'tasks' key (Anthropic style)."""
        data = {
            "tasks": [
                {"id": "task-1", "description": "First", "passes": False},
                {"id": "task-2", "description": "Second", "passes": True},
            ]
        }
        path = temp_project / "prd.json"
        path.write_text(json.dumps(data))

        tasks = load_json_tasks(str(path))
        assert len(tasks) == 1  # One is marked as passes=True
        assert tasks[0].id == "task-1"

    def test_object_with_items_key(self, temp_project: Path) -> None:
        """Test loading object with 'items' key."""
        data = {
            "items": [
                {"id": "item-1", "description": "First item"},
            ]
        }
        path = temp_project / "items.json"
        path.write_text(json.dumps(data))

        tasks = load_json_tasks(str(path))
        assert len(tasks) == 1
        assert tasks[0].id == "item-1"

    def test_priority_mapping(self, temp_project: Path) -> None:
        """Test priority is correctly mapped."""
        data = [
            {"id": "high", "description": "High", "priority": "high"},
            {"id": "med", "description": "Med", "priority": "medium"},
            {"id": "low", "description": "Low", "priority": "low"},
        ]
        path = temp_project / "tasks.json"
        path.write_text(json.dumps(data))

        tasks = load_json_tasks(str(path))
        assert tasks[0].priority == "high"
        assert tasks[1].priority == "medium"
        assert tasks[2].priority == "low"

    def test_alternative_description_fields(self, temp_project: Path) -> None:
        """Test alternative field names for description."""
        data = [
            {"id": "t1", "title": "Using title"},
            {"id": "t2", "summary": "Using summary"},
        ]
        path = temp_project / "tasks.json"
        path.write_text(json.dumps(data))

        tasks = load_json_tasks(str(path))
        assert tasks[0].description == "Using title"
        assert tasks[1].description == "Using summary"

    def test_generated_id(self, temp_project: Path) -> None:
        """Test ID is generated from description if missing."""
        data = [{"description": "Implement new feature"}]
        path = temp_project / "tasks.json"
        path.write_text(json.dumps(data))

        tasks = load_json_tasks(str(path))
        assert tasks[0].id == "implement-new-feature"

    def test_metadata_preserved(self, temp_project: Path) -> None:
        """Test original data is preserved as metadata."""
        data = [{"id": "task-1", "description": "Test", "custom_field": "value"}]
        path = temp_project / "tasks.json"
        path.write_text(json.dumps(data))

        tasks = load_json_tasks(str(path))
        assert tasks[0].metadata is not None
        assert tasks[0].metadata["custom_field"] == "value"

    def test_skips_empty_tasks(self, temp_project: Path) -> None:
        """Test that tasks without id and description are skipped."""
        data = [
            {"id": "", "description": ""},
            {"id": "valid", "description": "Valid task"},
        ]
        path = temp_project / "tasks.json"
        path.write_text(json.dumps(data))

        tasks = load_json_tasks(str(path))
        assert len(tasks) == 1
        assert tasks[0].id == "valid"

    def test_invalid_data_type(self, temp_project: Path) -> None:
        """Test with invalid data type (string)."""
        path = temp_project / "tasks.json"
        path.write_text('"just a string"')

        tasks = load_json_tasks(str(path))
        assert tasks == []

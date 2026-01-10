"""Tests for afk.sources.markdown module."""

from __future__ import annotations

from pathlib import Path

from afk.sources.markdown import (
    _generate_id,
    _parse_task_line,
    load_markdown_tasks,
)


class TestGenerateId:
    """Tests for _generate_id function."""

    def test_simple_description(self) -> None:
        """Test simple description."""
        assert _generate_id("Add new feature") == "add-new-feature"

    def test_long_description(self) -> None:
        """Test truncation of long descriptions."""
        long_desc = "This is a very long description that should be truncated"
        result = _generate_id(long_desc)
        assert len(result) <= 35

    def test_special_characters(self) -> None:
        """Test special characters are removed."""
        assert _generate_id("Fix bug #123!") == "fix-bug-123"

    def test_empty_description(self) -> None:
        """Test empty description."""
        assert _generate_id("") == "task"


class TestParseTaskLine:
    """Tests for _parse_task_line function."""

    def test_simple_task(self) -> None:
        """Test simple task without priority or ID."""
        task_id, desc, priority = _parse_task_line("Implement feature")
        assert desc == "Implement feature"
        assert priority == "medium"
        assert task_id == "implement-feature"

    def test_with_high_priority(self) -> None:
        """Test task with HIGH priority tag."""
        task_id, desc, priority = _parse_task_line("[HIGH] Critical task")
        assert desc == "Critical task"
        assert priority == "high"

    def test_with_critical_priority(self) -> None:
        """Test task with CRITICAL priority tag."""
        task_id, desc, priority = _parse_task_line("[CRITICAL] Very important")
        assert priority == "high"

    def test_with_low_priority(self) -> None:
        """Test task with LOW priority tag."""
        task_id, desc, priority = _parse_task_line("[LOW] Minor task")
        assert desc == "Minor task"
        assert priority == "low"

    def test_with_p0_priority(self) -> None:
        """Test task with P0 priority tag."""
        _, _, priority = _parse_task_line("[P0] Urgent")
        assert priority == "high"

    def test_with_p3_priority(self) -> None:
        """Test task with P3 priority tag."""
        _, _, priority = _parse_task_line("[P3] Not urgent")
        assert priority == "low"

    def test_with_explicit_id(self) -> None:
        """Test task with explicit ID."""
        task_id, desc, priority = _parse_task_line("my-task-id: Do the thing")
        assert task_id == "my-task-id"
        assert desc == "Do the thing"
        assert priority == "medium"

    def test_with_priority_and_id(self) -> None:
        """Test task with both priority and ID."""
        task_id, desc, priority = _parse_task_line("[HIGH] task-123: Important task")
        assert task_id == "task-123"
        assert desc == "Important task"
        assert priority == "high"

    def test_unknown_priority_tag(self) -> None:
        """Test task with unknown priority tag."""
        _, _, priority = _parse_task_line("[MEDIUM] Normal task")
        assert priority == "medium"


class TestLoadMarkdownTasks:
    """Tests for load_markdown_tasks function."""

    def test_no_path_no_default_files(self, temp_project: Path) -> None:
        """Test with no path and no default files."""
        tasks = load_markdown_tasks(None)
        assert tasks == []

    def test_no_path_finds_default(self, temp_project: Path) -> None:
        """Test finding default tasks.md."""
        (temp_project / "tasks.md").write_text("- [ ] Default task\n")
        tasks = load_markdown_tasks(None)
        assert len(tasks) == 1

    def test_no_path_finds_todo_md(self, temp_project: Path) -> None:
        """Test finding default TODO.md."""
        (temp_project / "TODO.md").write_text("- [ ] Todo item\n")
        tasks = load_markdown_tasks(None)
        assert len(tasks) == 1

    def test_file_not_found(self, temp_project: Path) -> None:
        """Test with non-existent file."""
        tasks = load_markdown_tasks("nonexistent.md")
        assert tasks == []

    def test_unchecked_checkbox(self, temp_project: Path) -> None:
        """Test parsing unchecked checkboxes."""
        content = "- [ ] Task one\n- [ ] Task two\n"
        path = temp_project / "tasks.md"
        path.write_text(content)

        tasks = load_markdown_tasks(str(path))
        assert len(tasks) == 2
        assert tasks[0].description == "Task one"
        assert tasks[0].source == f"markdown:{path}"

    def test_checked_checkbox_skipped(self, temp_project: Path) -> None:
        """Test that checked checkboxes are skipped."""
        content = "- [x] Completed\n- [ ] Pending\n"
        path = temp_project / "tasks.md"
        path.write_text(content)

        tasks = load_markdown_tasks(str(path))
        assert len(tasks) == 1
        assert tasks[0].description == "Pending"

    def test_uppercase_x_skipped(self, temp_project: Path) -> None:
        """Test that [X] is also skipped."""
        content = "- [X] Completed\n- [ ] Pending\n"
        path = temp_project / "tasks.md"
        path.write_text(content)

        tasks = load_markdown_tasks(str(path))
        assert len(tasks) == 1

    def test_asterisk_list_marker(self, temp_project: Path) -> None:
        """Test parsing * as list marker."""
        content = "* [ ] Task with asterisk\n"
        path = temp_project / "tasks.md"
        path.write_text(content)

        tasks = load_markdown_tasks(str(path))
        assert len(tasks) == 1
        assert tasks[0].description == "Task with asterisk"

    def test_indented_checkbox(self, temp_project: Path) -> None:
        """Test parsing indented checkboxes."""
        content = "  - [ ] Indented task\n    - [ ] More indented\n"
        path = temp_project / "tasks.md"
        path.write_text(content)

        tasks = load_markdown_tasks(str(path))
        assert len(tasks) == 2

    def test_with_priority_tags(self, temp_project: Path) -> None:
        """Test parsing priority tags."""
        content = """\
- [ ] [HIGH] High priority
- [ ] [LOW] Low priority
- [ ] Normal priority
"""
        path = temp_project / "tasks.md"
        path.write_text(content)

        tasks = load_markdown_tasks(str(path))
        assert len(tasks) == 3
        assert tasks[0].priority == "high"
        assert tasks[1].priority == "low"
        assert tasks[2].priority == "medium"

    def test_with_explicit_ids(self, temp_project: Path) -> None:
        """Test parsing explicit IDs."""
        content = """\
- [ ] task-1: First task
- [ ] task-2: Second task
"""
        path = temp_project / "tasks.md"
        path.write_text(content)

        tasks = load_markdown_tasks(str(path))
        assert len(tasks) == 2
        assert tasks[0].id == "task-1"
        assert tasks[0].description == "First task"
        assert tasks[1].id == "task-2"

    def test_mixed_content(self, temp_project: Path) -> None:
        """Test file with mixed content."""
        content = """\
# Project Tasks

Some introductory text.

## High Priority
- [ ] [HIGH] task-1: Critical fix
- [x] Already done

## Normal
- [ ] Regular task

Not a task: just text.
"""
        path = temp_project / "tasks.md"
        path.write_text(content)

        tasks = load_markdown_tasks(str(path))
        assert len(tasks) == 2
        assert tasks[0].id == "task-1"
        assert tasks[0].priority == "high"
        assert tasks[1].priority == "medium"

    def test_non_checkbox_lines_ignored(self, temp_project: Path) -> None:
        """Test that non-checkbox lines are ignored."""
        content = """\
- Regular list item
- Another item
- [ ] Actual task
"""
        path = temp_project / "tasks.md"
        path.write_text(content)

        tasks = load_markdown_tasks(str(path))
        assert len(tasks) == 1
        assert tasks[0].description == "Actual task"

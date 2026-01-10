"""Tests for afk.progress module."""

from __future__ import annotations

import json
from pathlib import Path

from afk.progress import (
    SessionProgress,
    TaskProgress,
    check_limits,
    mark_complete,
    mark_failed,
)


class TestTaskProgress:
    """Tests for TaskProgress model."""

    def test_defaults(self) -> None:
        """Test default values."""
        task = TaskProgress(id="task-1", source="beads")
        assert task.id == "task-1"
        assert task.source == "beads"
        assert task.status == "pending"
        assert task.started_at is None
        assert task.completed_at is None
        assert task.failure_count == 0
        assert task.commits == []
        assert task.message is None

    def test_all_fields(self) -> None:
        """Test all fields populated."""
        task = TaskProgress(
            id="task-2",
            source="json:tasks.json",
            status="completed",
            started_at="2025-01-10T10:00:00",
            completed_at="2025-01-10T10:30:00",
            failure_count=1,
            commits=["abc123", "def456"],
            message="Completed successfully",
        )
        assert task.status == "completed"
        assert task.failure_count == 1
        assert len(task.commits) == 2


class TestSessionProgress:
    """Tests for SessionProgress model."""

    def test_defaults(self) -> None:
        """Test default values."""
        session = SessionProgress()
        assert session.iterations == 0
        assert session.tasks == {}
        assert session.started_at is not None

    def test_load_missing_file(self, temp_afk_dir: Path) -> None:
        """Test loading returns new session when file doesn't exist."""
        session = SessionProgress.load(temp_afk_dir / "progress.json")
        assert session.iterations == 0
        assert session.tasks == {}

    def test_load_existing_file(self, temp_afk_dir: Path, sample_progress_data: dict) -> None:
        """Test loading from existing file."""
        progress_path = temp_afk_dir / "progress.json"
        with open(progress_path, "w") as f:
            json.dump(sample_progress_data, f)

        session = SessionProgress.load(progress_path)
        assert session.iterations == 3
        assert len(session.tasks) == 2
        assert session.tasks["task-1"].status == "completed"

    def test_save_creates_directory(self, temp_project: Path) -> None:
        """Test save creates parent directory if needed."""
        session = SessionProgress()
        session.iterations = 5
        progress_path = temp_project / ".afk" / "progress.json"
        session.save(progress_path)

        assert progress_path.exists()
        with open(progress_path) as f:
            data = json.load(f)
        assert data["iterations"] == 5

    def test_increment_iteration(self, temp_afk_dir: Path) -> None:
        """Test iteration increment."""
        progress_path = temp_afk_dir / "progress.json"
        session = SessionProgress()
        session.save(progress_path)

        # Increment
        count = session.increment_iteration()
        assert count == 1
        assert session.iterations == 1

        # Increment again
        count = session.increment_iteration()
        assert count == 2

    def test_get_task_exists(self, sample_progress_data: dict) -> None:
        """Test getting existing task."""
        session = SessionProgress.model_validate(sample_progress_data)
        task = session.get_task("task-1")
        assert task is not None
        assert task.status == "completed"

    def test_get_task_missing(self, sample_progress_data: dict) -> None:
        """Test getting non-existent task."""
        session = SessionProgress.model_validate(sample_progress_data)
        task = session.get_task("task-999")
        assert task is None

    def test_set_task_status_new(self, temp_afk_dir: Path) -> None:
        """Test setting status for new task."""
        progress_path = temp_afk_dir / "progress.json"
        session = SessionProgress()
        session.save(progress_path)

        task = session.set_task_status("task-new", "pending", source="beads")
        assert task.id == "task-new"
        assert task.source == "beads"
        assert task.status == "pending"

    def test_set_task_status_in_progress(self, temp_afk_dir: Path) -> None:
        """Test setting in_progress status sets started_at."""
        progress_path = temp_afk_dir / "progress.json"
        session = SessionProgress()
        session.save(progress_path)

        task = session.set_task_status("task-1", "in_progress")
        assert task.started_at is not None

    def test_set_task_status_completed(self, temp_afk_dir: Path) -> None:
        """Test setting completed status sets completed_at."""
        progress_path = temp_afk_dir / "progress.json"
        session = SessionProgress()
        session.save(progress_path)

        task = session.set_task_status("task-1", "completed", message="Done!")
        assert task.completed_at is not None
        assert task.message == "Done!"

    def test_set_task_status_failed_increments_count(self, temp_afk_dir: Path) -> None:
        """Test setting failed status increments failure_count."""
        progress_path = temp_afk_dir / "progress.json"
        session = SessionProgress()
        session.save(progress_path)

        session.set_task_status("task-1", "failed")
        assert session.tasks["task-1"].failure_count == 1

        session.set_task_status("task-1", "failed")
        assert session.tasks["task-1"].failure_count == 2

    def test_get_pending_tasks(self, sample_progress_data: dict) -> None:
        """Test getting pending tasks."""
        session = SessionProgress.model_validate(sample_progress_data)
        # Add a pending task
        session.tasks["task-3"] = TaskProgress(id="task-3", source="test", status="pending")

        pending = session.get_pending_tasks()
        assert len(pending) == 1
        assert pending[0].id == "task-3"

    def test_get_completed_tasks(self, sample_progress_data: dict) -> None:
        """Test getting completed tasks."""
        session = SessionProgress.model_validate(sample_progress_data)
        completed = session.get_completed_tasks()
        assert len(completed) == 1
        assert completed[0].id == "task-1"

    def test_is_complete_no_tasks(self) -> None:
        """Test is_complete with no tasks returns False."""
        session = SessionProgress()
        assert session.is_complete() is False

    def test_is_complete_all_done(self) -> None:
        """Test is_complete with all tasks completed/skipped."""
        session = SessionProgress()
        session.tasks["t1"] = TaskProgress(id="t1", source="test", status="completed")
        session.tasks["t2"] = TaskProgress(id="t2", source="test", status="skipped")
        assert session.is_complete() is True

    def test_is_complete_some_pending(self) -> None:
        """Test is_complete with pending tasks."""
        session = SessionProgress()
        session.tasks["t1"] = TaskProgress(id="t1", source="test", status="completed")
        session.tasks["t2"] = TaskProgress(id="t2", source="test", status="pending")
        assert session.is_complete() is False


class TestMarkComplete:
    """Tests for mark_complete function."""

    def test_mark_existing_task(self, temp_afk_dir: Path) -> None:
        """Test marking existing task as complete."""
        progress_path = temp_afk_dir / "progress.json"
        session = SessionProgress()
        session.tasks["task-1"] = TaskProgress(id="task-1", source="test")
        session.save(progress_path)

        result = mark_complete("task-1", message="All done")
        assert result is True

        # Verify saved
        loaded = SessionProgress.load(progress_path)
        assert loaded.tasks["task-1"].status == "completed"
        assert loaded.tasks["task-1"].message == "All done"

    def test_mark_new_task(self, temp_afk_dir: Path) -> None:
        """Test marking new task as complete creates it."""
        progress_path = temp_afk_dir / "progress.json"
        session = SessionProgress()
        session.save(progress_path)

        result = mark_complete("new-task")
        assert result is True

        loaded = SessionProgress.load(progress_path)
        assert "new-task" in loaded.tasks
        assert loaded.tasks["new-task"].status == "completed"


class TestMarkFailed:
    """Tests for mark_failed function."""

    def test_mark_failed_returns_count(self, temp_afk_dir: Path) -> None:
        """Test marking task as failed returns failure count."""
        progress_path = temp_afk_dir / "progress.json"
        session = SessionProgress()
        session.save(progress_path)

        count = mark_failed("task-1")
        assert count == 1

        count = mark_failed("task-1")
        assert count == 2


class TestCheckLimits:
    """Tests for check_limits function."""

    def test_under_limits(self, temp_afk_dir: Path) -> None:
        """Test when under all limits."""
        progress_path = temp_afk_dir / "progress.json"
        session = SessionProgress()
        session.iterations = 5
        session.save(progress_path)

        can_continue, reason = check_limits(max_iterations=10, max_failures=3, total_tasks=5)
        assert can_continue is True
        assert reason is None

    def test_iterations_exceeded(self, temp_afk_dir: Path) -> None:
        """Test when max iterations exceeded."""
        progress_path = temp_afk_dir / "progress.json"
        session = SessionProgress()
        session.iterations = 10
        session.save(progress_path)

        can_continue, reason = check_limits(max_iterations=10, max_failures=3)
        assert can_continue is False
        assert "AFK_LIMIT_REACHED" in reason  # type: ignore[operator]

    def test_all_tasks_complete(self, temp_afk_dir: Path) -> None:
        """Test when all tasks are complete."""
        progress_path = temp_afk_dir / "progress.json"
        session = SessionProgress()
        session.tasks["t1"] = TaskProgress(id="t1", source="test", status="completed")
        session.tasks["t2"] = TaskProgress(id="t2", source="test", status="completed")
        session.save(progress_path)

        can_continue, reason = check_limits(max_iterations=10, max_failures=3, total_tasks=2)
        assert can_continue is False
        assert "AFK_COMPLETE" in reason  # type: ignore[operator]

    def test_stuck_tasks_auto_skipped(self, temp_afk_dir: Path) -> None:
        """Test that tasks with too many failures are auto-skipped."""
        progress_path = temp_afk_dir / "progress.json"
        session = SessionProgress()
        session.tasks["stuck"] = TaskProgress(
            id="stuck", source="test", status="failed", failure_count=3
        )
        session.save(progress_path)

        can_continue, reason = check_limits(max_iterations=10, max_failures=3, total_tasks=2)
        # Should still be able to continue (task was skipped)
        assert can_continue is True

        # Verify task was skipped
        loaded = SessionProgress.load(progress_path)
        assert loaded.tasks["stuck"].status == "skipped"

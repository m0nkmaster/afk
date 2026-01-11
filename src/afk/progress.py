"""Progress tracking for afk."""

from __future__ import annotations

import json
from datetime import datetime
from pathlib import Path
from typing import Literal

from pydantic import BaseModel, Field

from afk.config import PROGRESS_FILE


class TaskProgress(BaseModel):
    """Progress record for a single task."""

    id: str
    source: str
    status: Literal["pending", "in_progress", "completed", "failed", "skipped"] = "pending"
    started_at: str | None = None
    completed_at: str | None = None
    failure_count: int = 0
    commits: list[str] = Field(default_factory=list)
    message: str | None = None
    learnings: list[str] = Field(default_factory=list)
    """Short-term learnings specific to this task, discovered during this session."""


class SessionProgress(BaseModel):
    """Progress for the current afk session."""

    started_at: str = Field(default_factory=lambda: datetime.now().isoformat())
    iterations: int = 0
    tasks: dict[str, TaskProgress] = Field(default_factory=dict)

    @classmethod
    def load(cls, path: Path | None = None) -> SessionProgress:
        """Load progress from file or return new session."""
        if path is None:
            path = PROGRESS_FILE

        if not path.exists():
            return cls()

        with open(path) as f:
            data = json.load(f)

        return cls.model_validate(data)

    def save(self, path: Path | None = None) -> None:
        """Save progress to file."""
        if path is None:
            path = PROGRESS_FILE

        path.parent.mkdir(parents=True, exist_ok=True)

        with open(path, "w") as f:
            json.dump(self.model_dump(), f, indent=2)

    def increment_iteration(self) -> int:
        """Increment and return the iteration count."""
        self.iterations += 1
        self.save()
        return self.iterations

    def get_task(self, task_id: str) -> TaskProgress | None:
        """Get a task by ID."""
        return self.tasks.get(task_id)

    def set_task_status(
        self,
        task_id: str,
        status: Literal["pending", "in_progress", "completed", "failed", "skipped"],
        source: str = "unknown",
        message: str | None = None,
    ) -> TaskProgress:
        """Set or update task status."""
        now = datetime.now().isoformat()

        if task_id not in self.tasks:
            self.tasks[task_id] = TaskProgress(id=task_id, source=source)

        task = self.tasks[task_id]
        task.status = status
        task.message = message

        if status == "in_progress" and not task.started_at:
            task.started_at = now
        elif status == "completed":
            task.completed_at = now
        elif status == "failed":
            task.failure_count += 1

        self.save()
        return task

    def get_pending_tasks(self) -> list[TaskProgress]:
        """Get all pending tasks."""
        return [t for t in self.tasks.values() if t.status == "pending"]

    def get_completed_tasks(self) -> list[TaskProgress]:
        """Get all completed tasks."""
        return [t for t in self.tasks.values() if t.status == "completed"]

    def is_complete(self) -> bool:
        """Check if all tasks are complete."""
        if not self.tasks:
            return False
        return all(t.status in ("completed", "skipped") for t in self.tasks.values())

    def add_learning(self, task_id: str, learning: str, source: str = "unknown") -> None:
        """Add a learning to a specific task.

        Args:
            task_id: The task ID to add the learning to
            learning: The learning content
            source: Source of the task (used if task doesn't exist yet)
        """
        if task_id not in self.tasks:
            self.tasks[task_id] = TaskProgress(id=task_id, source=source)

        self.tasks[task_id].learnings.append(learning)
        self.save()

    def get_all_learnings(self) -> dict[str, list[str]]:
        """Get all learnings grouped by task ID.

        Returns:
            Dict mapping task IDs to their learnings lists
        """
        return {
            task_id: task.learnings
            for task_id, task in self.tasks.items()
            if task.learnings
        }


def mark_complete(task_id: str, message: str | None = None) -> bool:
    """Mark a task as complete.

    Updates both progress.json and prd.json. If the task came from beads,
    it will also be closed in beads.
    """
    from afk.prd_store import mark_story_complete

    progress = SessionProgress.load()

    if task_id not in progress.tasks:
        # Create it if it doesn't exist (task might come from source directly)
        progress.set_task_status(task_id, "completed", message=message)
    else:
        progress.set_task_status(task_id, "completed", message=message)

    # Also mark complete in PRD (which handles beads sync-back)
    mark_story_complete(task_id)

    return True


def mark_failed(task_id: str, message: str | None = None) -> int:
    """Mark a task as failed, return failure count."""
    progress = SessionProgress.load()
    task = progress.set_task_status(task_id, "failed", message=message)
    return task.failure_count


def check_limits(
    max_iterations: int,
    max_failures: int,
    total_tasks: int = 0,
) -> tuple[bool, str | None]:
    """Check if limits have been reached.

    Args:
        max_iterations: Maximum number of iterations allowed
        max_failures: Maximum failures per task before skipping
        total_tasks: Total number of tasks from sources (for completion check)

    Returns (can_continue, reason_if_not).
    """
    progress = SessionProgress.load()

    if progress.iterations >= max_iterations:
        return False, f"AFK_LIMIT_REACHED: Max iterations ({max_iterations}) exceeded"

    # Check for tasks that have failed too many times
    stuck_tasks = [
        t
        for t in progress.tasks.values()
        if t.failure_count >= max_failures and t.status != "skipped"
    ]

    if stuck_tasks:
        # Auto-skip tasks that keep failing
        for task in stuck_tasks:
            progress.set_task_status(
                task.id, "skipped", message=f"Skipped after {task.failure_count} failures"
            )

    # Check completion - need to compare against total tasks from sources
    completed_count = len(progress.get_completed_tasks())
    skipped_count = len([t for t in progress.tasks.values() if t.status == "skipped"])

    if total_tasks > 0 and (completed_count + skipped_count) >= total_tasks:
        return False, "AFK_COMPLETE: All tasks finished"

    return True, None

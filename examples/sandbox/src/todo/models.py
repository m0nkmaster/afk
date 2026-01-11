"""Data models for the TODO app."""

from dataclasses import dataclass, field
from datetime import datetime
from enum import Enum


class Priority(Enum):
    """Task priority levels."""

    LOW = 1
    MEDIUM = 2
    HIGH = 3


@dataclass
class Task:
    """A single TODO task."""

    id: str
    title: str
    description: str = ""
    completed: bool = False
    priority: Priority = Priority.MEDIUM
    created_at: datetime = field(default_factory=datetime.now)

    def complete(self) -> None:
        """Mark the task as completed."""
        self.completed = True

    def __str__(self) -> str:
        """Return a string representation."""
        status = "✓" if self.completed else "○"
        return f"[{status}] {self.title}"


@dataclass
class TaskList:
    """A collection of tasks."""

    tasks: list[Task] = field(default_factory=list)

    def add(self, task: Task) -> None:
        """Add a task to the list."""
        self.tasks.append(task)

    def get(self, task_id: str) -> Task | None:
        """Get a task by ID."""
        for task in self.tasks:
            if task.id == task_id:
                return task
        return None

    def remove(self, task_id: str) -> bool:
        """Remove a task by ID. Returns True if removed."""
        # TODO: Implement this method
        pass

    def list_pending(self) -> list[Task]:
        """Return all incomplete tasks."""
        # TODO: Implement this method
        pass

    def list_by_priority(self, priority: Priority) -> list[Task]:
        """Return tasks matching the given priority."""
        # TODO: Implement this method
        pass

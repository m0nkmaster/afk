"""File system watcher for monitoring changes during iterations."""

from __future__ import annotations

from dataclasses import dataclass, field
from datetime import datetime
from fnmatch import fnmatch
from pathlib import Path
from typing import TYPE_CHECKING

from watchdog.events import (
    DirCreatedEvent,
    DirDeletedEvent,
    DirModifiedEvent,
    FileCreatedEvent,
    FileDeletedEvent,
    FileModifiedEvent,
    FileSystemEventHandler,
)
from watchdog.observers import Observer

if TYPE_CHECKING:
    from watchdog.events import FileSystemEvent


@dataclass
class FileChange:
    """Represents a single file change event.

    Attributes:
        path: Path to the file that changed.
        change_type: Type of change - 'created', 'modified', or 'deleted'.
        timestamp: When the change was detected.
    """

    path: str
    change_type: str
    timestamp: datetime = field(default_factory=datetime.now)


class _ChangeHandler(FileSystemEventHandler):
    """Internal handler that accumulates file change events."""

    def __init__(self, ignore_patterns: list[str] | None = None) -> None:
        """Initialise the handler.

        Args:
            ignore_patterns: Glob patterns for files/directories to ignore.
        """
        super().__init__()
        self._ignore_patterns = ignore_patterns or []
        self._changes: list[FileChange] = []

    def _should_ignore(self, path: str) -> bool:
        """Check if a path matches any ignore pattern.

        Args:
            path: File path to check.

        Returns:
            True if the path should be ignored.
        """
        for pattern in self._ignore_patterns:
            if fnmatch(path, pattern) or fnmatch(Path(path).name, pattern):
                return True
            # Also check if any path component matches
            for part in Path(path).parts:
                if fnmatch(part, pattern):
                    return True
        return False

    def _record_change(self, path: str, change_type: str) -> None:
        """Record a file change if not ignored.

        Args:
            path: Path to the changed file.
            change_type: Type of change.
        """
        if not self._should_ignore(path):
            self._changes.append(FileChange(path=path, change_type=change_type))

    def on_created(self, event: FileSystemEvent) -> None:
        """Handle file/directory creation events."""
        if isinstance(event, (FileCreatedEvent, DirCreatedEvent)):
            # Only track file creations, not directories
            if isinstance(event, FileCreatedEvent):
                path = event.src_path
                if isinstance(path, bytes):
                    path = path.decode("utf-8")
                self._record_change(path, "created")

    def on_modified(self, event: FileSystemEvent) -> None:
        """Handle file/directory modification events."""
        if isinstance(event, (FileModifiedEvent, DirModifiedEvent)):
            # Only track file modifications, not directories
            if isinstance(event, FileModifiedEvent):
                path = event.src_path
                if isinstance(path, bytes):
                    path = path.decode("utf-8")
                self._record_change(path, "modified")

    def on_deleted(self, event: FileSystemEvent) -> None:
        """Handle file/directory deletion events."""
        if isinstance(event, (FileDeletedEvent, DirDeletedEvent)):
            # Only track file deletions, not directories
            if isinstance(event, FileDeletedEvent):
                path = event.src_path
                if isinstance(path, bytes):
                    path = path.decode("utf-8")
                self._record_change(path, "deleted")

    def get_changes(self) -> list[FileChange]:
        """Get accumulated changes and clear the buffer.

        Returns:
            List of file changes since last call.
        """
        changes = self._changes.copy()
        self._changes.clear()
        return changes


class FileWatcher:
    """Watches a directory for file system changes using watchdog.

    This class provides a simple interface for monitoring file changes
    during an autonomous coding iteration. Changes are accumulated and
    can be retrieved via get_changes().

    Example:
        watcher = FileWatcher("/path/to/project", [".git", "__pycache__"])
        watcher.start()
        # ... do work ...
        changes = watcher.get_changes()
        watcher.stop()
    """

    def __init__(
        self,
        root: str | Path,
        ignore_patterns: list[str] | None = None,
    ) -> None:
        """Initialise the file watcher.

        Args:
            root: Root directory to watch.
            ignore_patterns: Glob patterns for files/directories to ignore.
                Defaults to common patterns like .git, __pycache__, etc.
        """
        self._root = Path(root)
        self._ignore_patterns = ignore_patterns or [
            ".git",
            "__pycache__",
            "*.pyc",
            ".afk",
            "node_modules",
            ".venv",
            "venv",
        ]
        self._handler = _ChangeHandler(self._ignore_patterns)
        self._observer = Observer()
        self._started = False

    def start(self) -> None:
        """Start watching for file system changes.

        This schedules the observer to watch the root directory recursively.
        Creates a new observer if the previous one was stopped (threads can
        only be started once).
        """
        if self._started:
            return

        # Create a fresh observer if needed (threads can only be started once)
        if not self._observer.is_alive():
            self._observer = Observer()

        self._observer.schedule(
            self._handler,
            str(self._root),
            recursive=True,
        )
        self._observer.start()
        self._started = True

    def stop(self) -> None:
        """Stop watching for file system changes.

        This stops the observer thread and waits for it to finish.
        """
        if not self._started:
            return

        self._observer.stop()
        self._observer.join()
        self._started = False

    def get_changes(self) -> list[FileChange]:
        """Get accumulated changes since last call.

        Returns:
            List of FileChange objects representing detected changes.
            The internal buffer is cleared after this call.
        """
        return self._handler.get_changes()

    @property
    def is_running(self) -> bool:
        """Check if the watcher is currently running."""
        return self._started

"""Feedback display and metrics for afk iterations."""

from __future__ import annotations

from dataclasses import dataclass, field
from datetime import datetime


@dataclass
class IterationMetrics:
    """Metrics collected during a single iteration.

    Tracks tool calls, file changes, line changes, and any errors or warnings
    encountered during an autonomous coding iteration.
    """

    tool_calls: int = 0
    """Number of tool calls made during the iteration."""

    files_modified: list[str] = field(default_factory=list)
    """List of file paths that were modified."""

    files_created: list[str] = field(default_factory=list)
    """List of file paths that were created."""

    files_deleted: list[str] = field(default_factory=list)
    """List of file paths that were deleted."""

    lines_added: int = 0
    """Total lines added across all file changes."""

    lines_removed: int = 0
    """Total lines removed across all file changes."""

    errors: list[str] = field(default_factory=list)
    """Error messages encountered during the iteration."""

    warnings: list[str] = field(default_factory=list)
    """Warning messages encountered during the iteration."""

    last_activity: datetime | None = None
    """Timestamp of the last detected activity."""


class MetricsCollector:
    """Accumulates metrics from parsed events during an iteration.

    This class provides methods to record tool calls, file changes, and other
    metrics during an autonomous coding iteration. The accumulated metrics can
    be accessed via the `metrics` property and reset between iterations.
    """

    def __init__(self) -> None:
        """Initialise the collector with empty metrics."""
        self._metrics = IterationMetrics()

    @property
    def metrics(self) -> IterationMetrics:
        """Access the current accumulated metrics."""
        return self._metrics

    def record_tool_call(self, tool_name: str) -> None:
        """Record that a tool was called.

        Args:
            tool_name: Name of the tool that was called.
        """
        self._metrics.tool_calls += 1
        self._metrics.last_activity = datetime.now()

    def record_file_change(self, path: str, change_type: str) -> None:
        """Record a file change event.

        Args:
            path: Path to the file that changed.
            change_type: Type of change - 'modified', 'created', or 'deleted'.
        """
        if change_type == "modified":
            self._metrics.files_modified.append(path)
        elif change_type == "created":
            self._metrics.files_created.append(path)
        elif change_type == "deleted":
            self._metrics.files_deleted.append(path)
        # Unknown change types are ignored but we still update activity
        self._metrics.last_activity = datetime.now()

    def reset(self) -> None:
        """Clear all accumulated metrics and start fresh."""
        self._metrics = IterationMetrics()

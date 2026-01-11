"""Feedback display and metrics for afk iterations."""

from __future__ import annotations

from dataclasses import dataclass, field
from datetime import datetime

from rich.console import Console, Group
from rich.live import Live
from rich.panel import Panel
from rich.text import Text

from afk.art import get_spinner_frame


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


class FeedbackDisplay:
    """Real-time feedback display using Rich Live panel.

    This class provides a live-updating terminal display that shows
    iteration progress, activity metrics, and status information
    during autonomous coding loops.
    """

    def __init__(self) -> None:
        """Initialise the feedback display."""
        self._console = Console()
        self._live: Live | None = None
        self._started = False
        self._spinner_frame: int = 0
        self._start_time: datetime | None = None
        self._iteration_current: int = 0
        self._iteration_total: int = 0

    def start(self) -> None:
        """Start the live display context.

        Initialises the Rich Live context for real-time updates.
        Safe to call multiple times; subsequent calls are no-ops.
        """
        if self._started:
            return

        self._start_time = datetime.now()
        self._live = Live(
            self._build_panel(),
            console=self._console,
            refresh_per_second=4,
            transient=True,
        )
        self._live.start()
        self._started = True

    def stop(self) -> None:
        """Stop the live display context.

        Cleanly exits the Rich Live context. Safe to call
        without having called start() first.
        """
        if self._live is not None:
            self._live.stop()
            self._started = False

    def _build_panel(self, metrics: IterationMetrics | None = None) -> Panel:
        """Build the main display panel.

        Args:
            metrics: Optional iteration metrics to display.

        Returns:
            A Rich renderable containing the feedback display.
        """
        header = Text()
        header.append("◉ ", style="green bold")
        header.append("afk", style="bold cyan")
        header.append(" running...", style="dim")

        # Add iteration info and elapsed time if available
        if self._iteration_total > 0:
            header.append(
                f"  Iteration {self._iteration_current}/{self._iteration_total}",
                style="cyan",
            )

        # Add elapsed time
        elapsed = self._format_elapsed_time()
        if elapsed:
            header.append(f"  {elapsed}", style="dim")

        if metrics is not None:
            activity_panel = self._build_activity_panel(metrics)
            files_panel = self._build_files_panel(metrics)
            content = Group(header, activity_panel, files_panel)
        else:
            content = Group(
                header,
                Text("Waiting for activity...", style="dim"),
            )

        return Panel(
            content,
            title="[bold]afk[/bold]",
            border_style="cyan",
        )

    def _format_elapsed_time(self) -> str:
        """Format elapsed time since start as mm:ss.

        Returns:
            Formatted time string, or empty string if start_time not set.
        """
        if self._start_time is None:
            return ""

        elapsed = datetime.now() - self._start_time
        total_seconds = int(elapsed.total_seconds())
        minutes = total_seconds // 60
        seconds = total_seconds % 60
        return f"{minutes:02d}:{seconds:02d}"

    def _build_activity_panel(self, metrics: IterationMetrics) -> Panel:
        """Build the activity panel showing spinner, tool calls, and line changes.

        Args:
            metrics: The current iteration metrics.

        Returns:
            A Rich Panel containing activity information.
        """
        # Get current spinner frame
        spinner = get_spinner_frame("dots", self._spinner_frame)

        # Build activity text
        activity = Text()
        activity.append(f"{spinner} ", style="cyan bold")
        activity.append("Working", style="bold")

        # Tool calls line
        tools_line = Text()
        tools_line.append("  Tools: ", style="dim")
        tools_line.append(str(metrics.tool_calls), style="yellow bold")

        # Files touched count (modified + created + deleted)
        files_touched = (
            len(metrics.files_modified) + len(metrics.files_created) + len(metrics.files_deleted)
        )
        files_line = Text()
        files_line.append("  Files: ", style="dim")
        files_line.append(str(files_touched), style="blue bold")

        # Lines added/removed
        lines_line = Text()
        lines_line.append("  Lines: ", style="dim")
        lines_line.append(f"+{metrics.lines_added}", style="green bold")
        lines_line.append(" / ", style="dim")
        lines_line.append(f"-{metrics.lines_removed}", style="red bold")

        content = Group(activity, tools_line, files_line, lines_line)

        return Panel(
            content,
            title="[dim]Activity[/dim]",
            border_style="dim",
        )

    def _build_files_panel(
        self, metrics: IterationMetrics, max_files: int = 5, max_path_length: int = 40
    ) -> Panel:
        """Build the files panel showing recently modified/created files.

        Args:
            metrics: The current iteration metrics.
            max_files: Maximum number of files to display (default 5).
            max_path_length: Maximum path length before truncation (default 40).

        Returns:
            A Rich Panel containing file change information.
        """
        lines: list[Text] = []

        # Collect files with their prefixes, taking most recent (last in list)
        # Created files come first, then modified - order matches how they appear
        file_entries: list[tuple[str, str]] = []

        for path in metrics.files_created:
            file_entries.append(("+", path))

        for path in metrics.files_modified:
            file_entries.append(("✎", path))

        # Take the most recent files (last N entries)
        recent_files = file_entries[-max_files:]

        for prefix, path in recent_files:
            line = Text()
            if prefix == "+":
                line.append(f"  {prefix} ", style="green bold")
            else:
                line.append(f"  {prefix} ", style="yellow")

            # Truncate long paths, keeping the filename visible
            display_path = self._truncate_path(path, max_path_length)
            line.append(display_path, style="dim")
            lines.append(line)

        if not lines:
            empty_line = Text()
            empty_line.append("  No files changed yet", style="dim italic")
            lines.append(empty_line)

        content = Group(*lines)

        return Panel(
            content,
            title="[dim]Files[/dim]",
            border_style="dim",
        )

    def _truncate_path(self, path: str, max_length: int) -> str:
        """Truncate a file path to fit within max_length.

        Preserves the filename and as much of the path as possible.

        Args:
            path: The file path to truncate.
            max_length: Maximum allowed length.

        Returns:
            The truncated path with ... if needed.
        """
        if len(path) <= max_length:
            return path

        # Split into directory and filename
        parts = path.rsplit("/", 1)
        if len(parts) == 1:
            # No directory, just truncate the filename
            return path[: max_length - 3] + "..."

        directory, filename = parts

        # Ensure filename fits, truncate directory if needed
        if len(filename) >= max_length - 4:
            # Filename alone is too long, truncate it
            return "..." + filename[-(max_length - 3) :]

        # Truncate directory to fit remaining space
        remaining = max_length - len(filename) - 4  # 4 for ".../"
        if remaining > 0:
            return "..." + directory[-remaining:] + "/" + filename
        else:
            return ".../" + filename

    def update(
        self,
        metrics: IterationMetrics,
        iteration_current: int = 0,
        iteration_total: int = 0,
    ) -> None:
        """Update the display with new metrics.

        Args:
            metrics: The current iteration metrics.
            iteration_current: Current iteration number (1-indexed).
            iteration_total: Total number of iterations planned.
        """
        # Update iteration tracking
        self._iteration_current = iteration_current
        self._iteration_total = iteration_total

        if self._live is None or not self._started:
            return

        # Increment spinner frame for animation
        self._spinner_frame += 1

        # Rebuild and update the display
        self._live.update(self._build_panel(metrics))

"""Feedback display and metrics for afk iterations."""

from __future__ import annotations

from dataclasses import dataclass, field
from datetime import datetime
from typing import Literal

from rich.console import Console, Group
from rich.live import Live
from rich.panel import Panel
from rich.progress_bar import ProgressBar
from rich.text import Text

from afk.art import get_mascot, get_spinner_frame

# Activity state thresholds (in seconds)
ACTIVE_THRESHOLD = 2.0  # Less than 2s since last activity = Active
THINKING_THRESHOLD = 10.0  # 2-10s = Thinking, >10s = Stalled


class ActivityState:
    """Constants for activity states."""

    ACTIVE = "active"
    THINKING = "thinking"
    STALLED = "stalled"


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

    def get_activity_state(self) -> str:
        """Determine the current activity state based on time since last activity.

        Returns:
            Activity state: 'active' (<2s), 'thinking' (2-10s), or 'stalled' (>10s).
            Returns 'active' if no activity has been recorded yet.
        """
        if self._metrics.last_activity is None:
            return ActivityState.ACTIVE

        elapsed = (datetime.now() - self._metrics.last_activity).total_seconds()

        if elapsed < ACTIVE_THRESHOLD:
            return ActivityState.ACTIVE
        elif elapsed < THINKING_THRESHOLD:
            return ActivityState.THINKING
        else:
            return ActivityState.STALLED


class FeedbackDisplay:
    """Real-time feedback display using Rich Live panel.

    This class provides a live-updating terminal display that shows
    iteration progress, activity metrics, and status information
    during autonomous coding loops.
    """

    def __init__(self, mode: Literal["full", "minimal"] = "full", show_mascot: bool = True) -> None:
        """Initialise the feedback display.

        Args:
            mode: Display mode - 'full' for multi-panel display,
                  'minimal' for single-line status bar.
            show_mascot: Whether to display the ASCII mascot panel.
        """
        self._console = Console()
        self._live: Live | None = None
        self._started = False
        self._spinner_frame: int = 0
        self._start_time: datetime | None = None
        self._iteration_current: int = 0
        self._iteration_total: int = 0
        self._mode: Literal["full", "minimal"] = mode
        self._show_mascot: bool = show_mascot
        self._task_id: str | None = None
        self._task_description: str | None = None
        self._progress: float = 0.0

    def start(self) -> None:
        """Start the live display context.

        Initialises the Rich Live context for real-time updates.
        Safe to call multiple times; subsequent calls are no-ops.
        """
        if self._started:
            return

        self._start_time = datetime.now()
        initial_renderable = (
            self._build_minimal_bar(IterationMetrics())
            if self._mode == "minimal"
            else self._build_panel()
        )
        self._live = Live(
            initial_renderable,
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

    def _build_panel(
        self,
        metrics: IterationMetrics | None = None,
        activity_state: str = ActivityState.ACTIVE,
    ) -> Panel:
        """Build the main display panel.

        Args:
            metrics: Optional iteration metrics to display.
            activity_state: Current activity state ('active', 'thinking', 'stalled').

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
            activity_panel = self._build_activity_panel(metrics, activity_state)
            files_panel = self._build_files_panel(metrics)
            content = Group(header, activity_panel, files_panel)
        else:
            content = Group(
                header,
                Text("Waiting for activity...", style="dim"),
            )

        # Add mascot panel if enabled
        if self._show_mascot:
            mascot_panel = self._build_mascot_panel(activity_state)
            content = Group(content, mascot_panel)

        # Add task panel as footer if task info available
        if self._task_id is not None:
            task_panel = self._build_task_panel()
            content = Group(content, task_panel)

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

    def _build_activity_panel(
        self, metrics: IterationMetrics, activity_state: str = ActivityState.ACTIVE
    ) -> Panel:
        """Build the activity panel showing spinner, tool calls, and line changes.

        Args:
            metrics: The current iteration metrics.
            activity_state: Current activity state ('active', 'thinking', 'stalled').

        Returns:
            A Rich Panel containing activity information.
        """
        # Get current spinner frame
        spinner = get_spinner_frame("dots", self._spinner_frame)

        # Determine spinner style and text based on activity state
        if activity_state == ActivityState.STALLED:
            spinner_style = "red bold"
            state_text = "Connection may be stalled..."
            state_style = "red bold"
        elif activity_state == ActivityState.THINKING:
            spinner_style = "yellow bold"
            state_text = "Thinking"
            state_style = "yellow bold"
        else:
            spinner_style = "cyan bold"
            state_text = "Working"
            state_style = "bold"

        # Build activity text
        activity = Text()
        activity.append(f"{spinner} ", style=spinner_style)
        activity.append(state_text, style=state_style)

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

    def _build_mascot_panel(self, activity_state: str = ActivityState.ACTIVE) -> Panel:
        """Build the mascot panel showing ASCII art character.

        Args:
            activity_state: Current activity state to determine mascot pose.

        Returns:
            A Rich Panel containing the mascot ASCII art.
        """
        # Map activity state to mascot state
        if activity_state == ActivityState.STALLED:
            mascot_state = "error"
        elif activity_state == ActivityState.THINKING:
            mascot_state = "waiting"
        else:
            mascot_state = "working"

        mascot_art = get_mascot(mascot_state)

        return Panel(
            Text(mascot_art, style="cyan"),
            border_style="dim",
        )

    def _build_task_panel(self) -> Panel:
        """Build the task panel showing current task and progress bar.

        Returns:
            A Rich Panel containing task information and progress bar.
        """
        lines: list[Text | ProgressBar] = []

        # Task ID line
        task_line = Text()
        task_line.append("  Task: ", style="dim")
        task_line.append(str(self._task_id or ""), style="cyan bold")
        lines.append(task_line)

        # Task description line (if available)
        if self._task_description:
            desc_line = Text()
            desc_line.append("  ", style="dim")
            # Truncate description if too long
            description = self._task_description
            if len(description) > 50:
                description = description[:47] + "..."
            desc_line.append(description, style="dim italic")
            lines.append(desc_line)

        # Progress bar
        progress_line = Text()
        progress_line.append("  ", style="dim")
        lines.append(progress_line)

        progress_bar = ProgressBar(
            total=100,
            completed=int(self._progress * 100),
            width=40,
            complete_style="green",
            finished_style="green bold",
        )
        lines.append(progress_bar)

        # Percentage text
        pct_line = Text()
        pct_line.append(f"  {int(self._progress * 100)}% complete", style="dim")
        lines.append(pct_line)

        content = Group(*lines)

        return Panel(
            content,
            title="[dim]Task[/dim]",
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
        task_id: str | None = None,
        task_description: str | None = None,
        progress: float = 0.0,
        activity_state: str = ActivityState.ACTIVE,
    ) -> None:
        """Update the display with new metrics.

        Args:
            metrics: The current iteration metrics.
            iteration_current: Current iteration number (1-indexed).
            iteration_total: Total number of iterations planned.
            task_id: ID of the current task being worked on.
            task_description: Description of the current task.
            progress: Task completion percentage (0.0 to 1.0).
            activity_state: Current activity state ('active', 'thinking', 'stalled').
        """
        # Update iteration tracking
        self._iteration_current = iteration_current
        self._iteration_total = iteration_total

        # Update task tracking
        self._task_id = task_id
        self._task_description = task_description
        self._progress = max(0.0, min(1.0, progress))  # Clamp to [0.0, 1.0]

        if self._live is None or not self._started:
            return

        # Increment spinner frame for animation
        self._spinner_frame += 1

        # Rebuild and update the display using appropriate mode
        if self._mode == "minimal":
            self._live.update(self._build_minimal_bar(metrics, activity_state))
        else:
            self._live.update(self._build_panel(metrics, activity_state))

    def _build_minimal_bar(
        self, metrics: IterationMetrics, activity_state: str = ActivityState.ACTIVE
    ) -> Text:
        """Build the minimal mode single-line status bar.

        Format: ◉ afk [x/y] mm:ss │ ⣾ N calls │ N files │ +N/-N

        Args:
            metrics: The current iteration metrics.
            activity_state: Current activity state ('active', 'thinking', 'stalled').

        Returns:
            A Rich Text object containing the status bar.
        """
        bar = Text()

        # Prefix: ◉ afk
        bar.append("◉ ", style="green bold")
        bar.append("afk", style="bold cyan")

        # Iteration count: [x/y]
        if self._iteration_total > 0:
            bar.append(f" [{self._iteration_current}/{self._iteration_total}]", style="cyan")

        # Elapsed time: mm:ss
        elapsed = self._format_elapsed_time()
        if elapsed:
            bar.append(f" {elapsed}", style="dim")

        # Separator
        bar.append(" │ ", style="dim")

        # Spinner with colour based on activity state
        spinner = get_spinner_frame("dots", self._spinner_frame)
        if activity_state == ActivityState.STALLED:
            bar.append(f"{spinner} ", style="red bold")
            bar.append("stalled? ", style="red")
        elif activity_state == ActivityState.THINKING:
            bar.append(f"{spinner} ", style="yellow bold")
        else:
            bar.append(f"{spinner} ", style="cyan bold")
        bar.append(f"{metrics.tool_calls} calls", style="yellow")

        # Separator
        bar.append(" │ ", style="dim")

        # Files count
        files_count = (
            len(metrics.files_modified) + len(metrics.files_created) + len(metrics.files_deleted)
        )
        bar.append(f"{files_count} files", style="blue")

        # Separator
        bar.append(" │ ", style="dim")

        # Line changes
        bar.append(f"+{metrics.lines_added}", style="green bold")
        bar.append("/", style="dim")
        bar.append(f"-{metrics.lines_removed}", style="red bold")

        return bar

    def show_gates_failed(self, failed_gates: list[str], continuing: bool = True) -> None:
        """Display visual feedback when quality gates fail.

        Shows an orange/red warning with the names of failed gates, and
        optionally indicates that the loop is continuing.

        Args:
            failed_gates: List of names of gates that failed.
            continuing: If True, show 'Continuing...' indicator.
        """
        # Build the warning message
        warning = Text()
        warning.append("⚠ ", style="yellow bold")
        warning.append("Quality gates failed: ", style="red bold")
        warning.append(", ".join(failed_gates), style="red")

        if continuing:
            warning.append(" │ ", style="dim")
            warning.append("Continuing...", style="yellow")

        self._console.print(warning)

    def show_celebration(self, task_id: str) -> None:
        """Display a celebration animation when a task is completed.

        Shows the celebration mascot with stars and a completion message.
        Uses a brief animation sequence for visual feedback.

        Args:
            task_id: The ID of the task that was completed.
        """
        import time

        # Animation frames using celebration mascot
        celebration_art = get_mascot("celebration")

        # Build the celebration panel content
        content_lines: list[Text] = []

        # Add star border
        star_line = Text()
        star_line.append("  ★ " * 8, style="yellow bold")
        content_lines.append(star_line)

        # Add empty line
        content_lines.append(Text())

        # Add mascot art
        mascot_text = Text()
        mascot_text.append(celebration_art, style="green bold")
        content_lines.append(mascot_text)

        # Add empty line
        content_lines.append(Text())

        # Add completion message
        message = Text()
        message.append("  ✓ Task Complete! ", style="green bold")
        message.append(task_id, style="cyan bold")
        content_lines.append(message)

        # Add empty line
        content_lines.append(Text())

        # Add star border again
        content_lines.append(star_line)

        content = Group(*content_lines)

        # Create celebration panel
        panel = Panel(
            content,
            title="[bold green]🎉 Celebration 🎉[/bold green]",
            border_style="green",
        )

        # Brief animation: show the panel with a short delay
        self._console.print()  # Add spacing
        self._console.print(panel)
        time.sleep(0.5)  # Brief pause to let the user see the celebration

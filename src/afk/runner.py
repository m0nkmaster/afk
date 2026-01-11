"""Runner for autonomous afk loops.

This module implements the Ralph Wiggum pattern: spawning fresh AI CLI
instances for each iteration, ensuring clean context between runs.

Architecture:
    OutputHandler - Console output and completion signal detection
    IterationRunner - Single iteration execution with streaming
    LoopController - Loop management, limits, archiving
"""

from __future__ import annotations

import subprocess
import time
from collections.abc import Callable
from dataclasses import dataclass, field
from datetime import datetime, timedelta
from enum import Enum
from pathlib import Path

from rich.console import Console
from rich.panel import Panel

from afk.config import AfkConfig, FeedbackLoopsConfig
from afk.feedback import FeedbackDisplay, MetricsCollector
from afk.file_watcher import FileWatcher
from afk.git_ops import archive_session, clear_session, create_branch
from afk.output_parser import (
    ErrorEvent,
    FileChangeEvent,
    OutputParser,
    ToolCallEvent,
    WarningEvent,
)
from afk.prd_store import (
    PrdDocument,
    UserStory,
    all_stories_complete,
    get_pending_stories,
    load_prd,
    sync_prd,
)
from afk.progress import SessionProgress, check_limits
from afk.prompt import generate_prompt

# Completion signals to detect in AI output (ralf.sh style)
COMPLETION_SIGNALS = [
    "<promise>COMPLETE</promise>",
    "AFK_COMPLETE",
    "AFK_STOP",
]


class StopReason(Enum):
    """Reasons for stopping the runner."""

    COMPLETE = "All tasks completed"
    MAX_ITERATIONS = "Maximum iterations reached"
    TIMEOUT = "Session timeout reached"
    NO_TASKS = "No tasks available"
    USER_INTERRUPT = "User interrupted"
    AI_ERROR = "AI CLI error"


@dataclass
class QualityGateResult:
    """Result of running quality gates."""

    passed: bool
    failed_gates: list[str]
    output: dict[str, str]


@dataclass
class RunResult:
    """Result of a runner session."""

    iterations_completed: int
    tasks_completed: int
    stop_reason: StopReason
    duration_seconds: float
    archived_to: Path | None = None


@dataclass
class IterationResult:
    """Result of a single iteration."""

    success: bool
    task_id: str | None = None
    error: str | None = None
    output: str = ""


# =============================================================================
# OutputHandler - Console output and completion signal detection
# =============================================================================


class OutputHandler:
    """Handles console output and completion signal detection.

    Encapsulates all Rich console interactions and signal detection logic.
    Also integrates OutputParser and MetricsCollector for real-time
    metrics tracking during iteration output streaming.
    """

    def __init__(
        self,
        console: Console | None = None,
        completion_signals: list[str] | None = None,
        feedback_enabled: bool = False,
        feedback_mode: str = "full",
        watch_root: str | Path | None = None,
        watch_ignore_patterns: list[str] | None = None,
        show_mascot: bool = True,
    ) -> None:
        """Initialise output handler.

        Args:
            console: Rich Console instance (creates default if None)
            completion_signals: Signals to detect for early termination
            feedback_enabled: Whether to show real-time feedback display
            feedback_mode: Feedback display mode ('full', 'minimal', 'off')
            watch_root: Root directory to watch for file changes (enables FileWatcher)
            watch_ignore_patterns: Glob patterns to ignore in file watching
            show_mascot: Whether to display ASCII mascot in feedback display
        """
        self.console = console or Console()
        self.signals = completion_signals or COMPLETION_SIGNALS
        self._parser = OutputParser()
        self._collector = MetricsCollector()
        self._feedback_enabled = feedback_enabled and feedback_mode != "off"
        self._feedback_mode = feedback_mode
        self._show_mascot = show_mascot
        self._feedback: FeedbackDisplay | None = None
        if self._feedback_enabled:
            # Cast mode to Literal type for FeedbackDisplay
            mode = "minimal" if feedback_mode == "minimal" else "full"
            self._feedback = FeedbackDisplay(mode=mode, show_mascot=show_mascot)  # type: ignore[arg-type]
        # FileWatcher for backup file change detection
        self._file_watcher: FileWatcher | None = None
        if watch_root is not None:
            self._file_watcher = FileWatcher(watch_root, watch_ignore_patterns)
        # Track paths already recorded to avoid duplicates
        self._recorded_paths: set[str] = set()

    @property
    def metrics_collector(self) -> MetricsCollector:
        """Access the metrics collector for reading accumulated metrics."""
        return self._collector

    @property
    def output_parser(self) -> OutputParser:
        """Access the output parser for external use if needed."""
        return self._parser

    @property
    def feedback(self) -> FeedbackDisplay | None:
        """Access the feedback display if enabled."""
        return self._feedback

    @property
    def file_watcher(self) -> FileWatcher | None:
        """Access the file watcher if configured."""
        return self._file_watcher

    def start_feedback(self) -> None:
        """Start the feedback display and file watcher if enabled.

        Should be called when an iteration begins.
        """
        if self._feedback is not None:
            self._feedback.start()
        if self._file_watcher is not None:
            self._file_watcher.start()
            # Clear any stale changes from previous runs
            self._file_watcher.get_changes()
        # Reset recorded paths for the new iteration
        self._recorded_paths.clear()

    def stop_feedback(self) -> None:
        """Stop the feedback display and file watcher if enabled.

        Should be called when an iteration completes. Polls any remaining
        file changes from the watcher and records them before stopping.
        """
        # Poll final changes from file watcher before stopping
        if self._file_watcher is not None:
            self._poll_watcher_changes()
            self._file_watcher.stop()
        if self._feedback is not None:
            self._feedback.stop()

    def _poll_watcher_changes(self) -> None:
        """Poll file watcher for changes and record in metrics collector.

        Deduplicates changes by path to avoid double-counting files that
        were already recorded from parsed output events.
        """
        if self._file_watcher is None:
            return

        changes = self._file_watcher.get_changes()
        for change in changes:
            # Deduplicate: only record if path not already recorded
            if change.path not in self._recorded_paths:
                self._collector.record_file_change(change.path, change.change_type)
                self._recorded_paths.add(change.path)

    def contains_completion_signal(self, output: str | None) -> bool:
        """Check if output contains any completion signal."""
        if not output:
            return False
        return any(signal in output for signal in self.signals)

    def iteration_header(self, iteration: int, max_iterations: int | None = None) -> None:
        """Display iteration header."""
        if max_iterations:
            self.console.print(f"\n[cyan]━━━ Iteration {iteration}/{max_iterations} ━━━[/cyan]")
        else:
            self.console.print(f"\n[cyan]━━━ Iteration {iteration} ━━━[/cyan]")

    def command_info(self, cmd: list[str]) -> None:
        """Display command being run."""
        self.console.print(f"[dim]Running: {' '.join(cmd)}[/dim]")
        self.console.print()

    def stream_line(self, line: str) -> None:
        """Output a streamed line and parse for metrics events.

        Parses the line through OutputParser and records any detected
        events (tool calls, file changes, errors, warnings) in the
        MetricsCollector. Also polls the file watcher for backup detection
        of file changes. Updates the feedback display if enabled.

        Args:
            line: A line of output from the AI CLI.
        """
        self.console.print(line, end="")

        # Parse line and record events in metrics collector
        events = self._parser.parse(line)
        for event in events:
            if isinstance(event, ToolCallEvent):
                self._collector.record_tool_call(event.tool_name)
            elif isinstance(event, FileChangeEvent):
                self._collector.record_file_change(event.file_path, event.change_type)
                # Track path to avoid duplicates from file watcher
                self._recorded_paths.add(event.file_path)
            elif isinstance(event, ErrorEvent):
                self._collector.metrics.errors.append(event.error_message)
                self._collector.metrics.last_activity = datetime.now()
            elif isinstance(event, WarningEvent):
                self._collector.metrics.warnings.append(event.warning_message)
                self._collector.metrics.last_activity = datetime.now()

        # Poll file watcher for backup file change detection
        self._poll_watcher_changes()

        # Update feedback display with current metrics
        if self._feedback is not None:
            self._feedback.update(self._collector.metrics)

    def completion_detected(self) -> None:
        """Display completion signal detected message."""
        self.console.print()
        self.console.print("[green]Completion signal detected![/green]")

    def error(self, message: str) -> None:
        """Display an error message."""
        self.console.print(f"[red]{message}[/red]")

    def warning(self, message: str) -> None:
        """Display a warning message."""
        self.console.print(f"[yellow]{message}[/yellow]")

    def success(self, message: str) -> None:
        """Display a success message."""
        self.console.print(f"[green]{message}[/green]")

    def info(self, message: str) -> None:
        """Display an info message."""
        self.console.print(f"[cyan]{message}[/cyan]")

    def dim(self, message: str) -> None:
        """Display a dimmed message."""
        self.console.print(f"[dim]{message}[/dim]")

    def show_gates_failed(self, failed_gates: list[str], continuing: bool = True) -> None:
        """Display quality gates failure feedback.

        Uses FeedbackDisplay if available, otherwise falls back to console output.

        Args:
            failed_gates: List of names of gates that failed.
            continuing: If True, show 'Continuing...' indicator.
        """
        if self._feedback is not None:
            self._feedback.show_gates_failed(failed_gates, continuing)
        else:
            # Fallback: print directly to console
            msg = f"Quality gates failed: {', '.join(failed_gates)}"
            if continuing:
                msg += " │ Continuing..."
            self.console.print(f"[red]{msg}[/red]")

    def show_celebration(self, task_id: str) -> None:
        """Display celebration animation when a task is completed.

        Uses FeedbackDisplay if available, otherwise falls back to console output.

        Args:
            task_id: The ID of the task that was completed.
        """
        if self._feedback is not None:
            self._feedback.show_celebration(task_id)
        else:
            # Fallback: print simple celebration message to console
            self.console.print()
            self.console.print(f"[green bold]✓ Task Complete![/green bold] [cyan]{task_id}[/cyan]")
            self.console.print()

    def show_session_complete(
        self, tasks_completed: int, iterations: int, duration_seconds: float
    ) -> None:
        """Display session complete celebration with summary statistics.

        Uses FeedbackDisplay if available, otherwise falls back to console output.

        Args:
            tasks_completed: Number of tasks completed in this session.
            iterations: Number of iterations run.
            duration_seconds: Total session duration in seconds.
        """
        if self._feedback is not None:
            self._feedback.show_session_complete(tasks_completed, iterations, duration_seconds)
        else:
            # Fallback: print simple session complete message to console
            minutes = int(duration_seconds) // 60
            seconds = int(duration_seconds) % 60
            self.console.print()
            self.console.print("[green bold]✓ All Tasks Complete![/green bold]")
            self.console.print(f"  Tasks: [cyan]{tasks_completed}[/cyan]")
            self.console.print(f"  Iterations: [cyan]{iterations}[/cyan]")
            self.console.print(f"  Time: [cyan]{minutes}m {seconds}s[/cyan]")
            self.console.print()

    def show_gates_passed(self, gates: list[str]) -> None:
        """Display visual feedback when quality gates pass successfully.

        Uses FeedbackDisplay if available, otherwise falls back to console output.

        Args:
            gates: List of names of gates that passed.
        """
        if self._feedback is not None:
            self._feedback.show_gates_passed(gates)
        else:
            # Fallback: print simple success message to console
            for gate in gates:
                self.console.print(f"  [green]✓[/green] {gate} passed")

    def loop_start_panel(
        self,
        ai_cli: str,
        max_iterations: int | str,
        timeout_minutes: int,
        pending_count: int,
        total_count: int,
    ) -> None:
        """Display the loop start panel."""
        self.console.print(
            Panel.fit(
                f"[bold]Starting afk loop[/bold]\n\n"
                f"AI CLI: [cyan]{ai_cli}[/cyan]\n"
                f"Max iterations: [cyan]{max_iterations}[/cyan]\n"
                f"Timeout: [cyan]{timeout_minutes} minutes[/cyan]\n"
                f"Stories: [cyan]{pending_count}/{total_count} pending[/cyan]",
                title="afk run",
            )
        )

    def session_complete_panel(
        self,
        iterations: int,
        tasks_completed: int,
        duration: float,
        stop_reason: StopReason,
        archived_to: Path | None = None,
    ) -> None:
        """Display session complete panel."""
        content = (
            f"[bold]Session Complete[/bold]\n\n"
            f"Iterations: [cyan]{iterations}[/cyan]\n"
            f"Tasks completed: [cyan]{tasks_completed}[/cyan]\n"
            f"Duration: [cyan]{duration:.1f}s[/cyan]\n"
            f"Reason: [cyan]{stop_reason.value}[/cyan]"
        )
        if archived_to:
            content += f"\nArchived to: [dim]{archived_to}[/dim]"

        self.console.print(Panel.fit(content, title="afk"))


# =============================================================================
# IterationRunner - Single iteration execution
# =============================================================================


@dataclass
class IterationRunner:
    """Runs a single iteration with fresh AI context.

    Handles spawning AI CLI, streaming output, and detecting completion signals.
    """

    config: AfkConfig
    output: OutputHandler = field(default_factory=OutputHandler)

    def run(
        self,
        iteration: int,
        prompt: str | None = None,
        on_output: Callable[[str], None] | None = None,
        stream: bool = True,
    ) -> IterationResult:
        """Run a single iteration.

        Args:
            iteration: Current iteration number
            prompt: Prompt content (generates if None)
            on_output: Optional callback for streaming output
            stream: If True, stream output in real-time

        Returns:
            IterationResult with success status and any output
        """
        # Generate prompt if not provided
        if prompt is None:
            prompt = generate_prompt(self.config, bootstrap=True)

        # Check for stop signals in prompt
        if "AFK_COMPLETE" in prompt:
            return IterationResult(success=True, error="AFK_COMPLETE")
        if "AFK_LIMIT_REACHED" in prompt:
            return IterationResult(success=False, error="AFK_LIMIT_REACHED")

        # Build command
        cmd = [self.config.ai_cli.command] + self.config.ai_cli.args

        self.output.iteration_header(iteration)
        self.output.command_info(cmd)

        return self._execute_command(cmd, prompt, on_output, stream)

    def _execute_command(
        self,
        cmd: list[str],
        prompt: str,
        on_output: Callable[[str], None] | None,
        stream: bool,
    ) -> IterationResult:
        """Execute AI CLI command and return result."""
        try:
            # Pass prompt as final argument (universal across AI CLIs)
            full_cmd = cmd + [prompt]

            process = subprocess.Popen(
                full_cmd,
                stdin=subprocess.DEVNULL,
                stdout=subprocess.PIPE,
                stderr=subprocess.STDOUT,
                text=True,
                bufsize=1,
            )

            if stream and process.stdout:
                output_buffer: list[str] = []
                for line in iter(process.stdout.readline, ""):
                    self.output.stream_line(line)
                    output_buffer.append(line)

                    if self.output.contains_completion_signal(line):
                        self.output.completion_detected()
                        process.terminate()
                        return IterationResult(success=True, output="".join(output_buffer))

                    if on_output:
                        on_output(line)

                stdout = "".join(output_buffer)
            else:
                stdout, _ = process.communicate(timeout=self.config.limits.timeout_minutes * 60)
                if on_output and stdout:
                    on_output(stdout)

            process.wait()

            if process.returncode != 0:
                return IterationResult(
                    success=False,
                    error=f"AI CLI exited with code {process.returncode}",
                    output=stdout or "",
                )

            return IterationResult(success=True, output=stdout or "")

        except subprocess.TimeoutExpired:
            process.kill()
            return IterationResult(success=False, error="Iteration timed out")
        except FileNotFoundError:
            return IterationResult(
                success=False,
                error=f"AI CLI not found: {self.config.ai_cli.command}",
            )
        except Exception as e:
            return IterationResult(success=False, error=str(e))


# =============================================================================
# LoopController - Loop management, limits, archiving
# =============================================================================


@dataclass
class LoopController:
    """Controls the main loop lifecycle.

    Manages limits, archiving, PRD sync, and stop conditions.
    """

    config: AfkConfig
    output: OutputHandler = field(default_factory=OutputHandler)
    iteration_runner: IterationRunner | None = None

    def __post_init__(self) -> None:
        """Initialise iteration runner if not provided."""
        if self.iteration_runner is None:
            self.iteration_runner = IterationRunner(self.config, self.output)

    def run(
        self,
        max_iterations: int | None = None,
        branch: str | None = None,
        until_complete: bool = False,
        timeout_override: int | None = None,
        on_iteration_complete: Callable[[int, IterationResult], None] | None = None,
        resume: bool = False,
        feedback_mode: str | None = None,
        show_mascot: bool | None = None,
    ) -> RunResult:
        """Run the autonomous loop.

        Args:
            max_iterations: Override for max iterations
            branch: Branch name to create/checkout
            until_complete: If True, run until all tasks done
            timeout_override: Override timeout in minutes
            on_iteration_complete: Callback after each iteration
            resume: If True, continue from last session
            feedback_mode: Feedback display mode ('full', 'minimal', 'off')
            show_mascot: Whether to display ASCII mascot (default: from config)

        Returns:
            RunResult with session statistics
        """
        # Configure feedback from parameter or config
        effective_feedback_mode = feedback_mode or self.config.feedback.mode
        feedback_enabled = self.config.feedback.enabled and effective_feedback_mode != "off"

        # Determine mascot visibility from parameter or config
        effective_show_mascot = (
            show_mascot if show_mascot is not None else self.config.feedback.show_mascot
        )

        # Recreate output handler with feedback settings
        self.output = OutputHandler(
            feedback_enabled=feedback_enabled,
            feedback_mode=effective_feedback_mode,
            watch_root=".",  # Watch current directory for file changes
            show_mascot=effective_show_mascot,
        )
        # Update iteration runner to use the new output handler
        self.iteration_runner = IterationRunner(self.config, self.output)

        start_time = datetime.now()
        timeout_minutes = timeout_override or self.config.limits.timeout_minutes
        timeout_delta = timedelta(minutes=timeout_minutes)
        max_iter = max_iterations or self.config.limits.max_iterations

        # Handle branching
        self._handle_branching(branch)

        # Handle archiving/resume
        self._handle_session_start(resume)

        # Sync PRD
        self.output.dim("Syncing PRD from sources...")
        prd = sync_prd(self.config, branch_name=branch)
        pending = get_pending_stories(prd)

        self.output.loop_start_panel(
            ai_cli=self.config.ai_cli.command,
            max_iterations="∞" if until_complete else max_iter,
            timeout_minutes=timeout_minutes,
            pending_count=len(pending),
            total_count=len(prd.user_stories),
        )

        return self._run_loop(
            start_time=start_time,
            timeout_delta=timeout_delta,
            max_iter=max_iter,
            until_complete=until_complete,
            on_iteration_complete=on_iteration_complete,
            prd=prd,
        )

    def _handle_branching(self, branch: str | None) -> None:
        """Handle git branching if configured."""
        if branch and self.config.git.auto_branch:
            full_branch = f"{self.config.git.branch_prefix}{branch}"
            self.output.info(f"Creating/switching to branch: {full_branch}")
            create_branch(branch, self.config)

    def _handle_session_start(self, resume: bool) -> None:
        """Handle session archiving or resumption."""
        if self.config.archive.enabled and not resume:
            progress = SessionProgress.load()
            if progress.iterations > 0:
                archive_path = archive_session(self.config, reason="new_run")
                if archive_path:
                    self.output.dim(f"Archived previous session to: {archive_path}")
                clear_session()
        elif resume:
            progress = SessionProgress.load()
            if progress.iterations > 0:
                self.output.info(f"Resuming session from iteration {progress.iterations}")

    def _run_loop(
        self,
        start_time: datetime,
        timeout_delta: timedelta,
        max_iter: int,
        until_complete: bool,
        on_iteration_complete: Callable[[int, IterationResult], None] | None,
        prd: PrdDocument,
    ) -> RunResult:
        """Execute the main loop."""
        iterations_completed = 0
        tasks_completed_this_session = 0
        stop_reason = StopReason.COMPLETE
        archived_to = None

        try:
            while True:
                # Check stop conditions
                stop_reason, should_break = self._check_stop_conditions(
                    start_time, timeout_delta, iterations_completed, max_iter, until_complete
                )
                if should_break:
                    break

                # Check PRD completion
                prd = load_prd()
                stop_reason, should_break = self._check_prd_completion(prd)
                if should_break:
                    break

                # Check limits
                stop_reason, should_break = self._check_limits(max_iter, until_complete, prd)
                if should_break:
                    break

                # Mark current task as in_progress in progress.json
                pending = get_pending_stories(prd)
                if pending:
                    self._mark_task_in_progress(pending[0])

                # Run iteration
                iteration_num = iterations_completed + 1
                assert self.iteration_runner is not None  # Set in __post_init__
                result = self.iteration_runner.run(iteration_num)

                if on_iteration_complete:
                    on_iteration_complete(iteration_num, result)

                # Handle result
                stop_reason, should_break = self._handle_iteration_result(result)
                if should_break:
                    break

                iterations_completed += 1

                # Check for newly completed stories and update progress.json
                tasks_completed_this_session += self._check_story_completion(prd)
                prd = load_prd()

                time.sleep(1)

        except KeyboardInterrupt:
            stop_reason = StopReason.USER_INTERRUPT
            self.output.warning("Interrupted by user")

        # Archive final session
        if self.config.archive.enabled:
            archived_to = archive_session(self.config, reason=stop_reason.name.lower())

        duration = (datetime.now() - start_time).total_seconds()

        # Show session complete celebration when all tasks finished
        if stop_reason == StopReason.COMPLETE and tasks_completed_this_session > 0:
            self.output.show_session_complete(
                tasks_completed=tasks_completed_this_session,
                iterations=iterations_completed,
                duration_seconds=duration,
            )

        self.output.session_complete_panel(
            iterations=iterations_completed,
            tasks_completed=tasks_completed_this_session,
            duration=duration,
            stop_reason=stop_reason,
            archived_to=archived_to,
        )

        return RunResult(
            iterations_completed=iterations_completed,
            tasks_completed=tasks_completed_this_session,
            stop_reason=stop_reason,
            duration_seconds=duration,
            archived_to=archived_to,
        )

    def _check_stop_conditions(
        self,
        start_time: datetime,
        timeout_delta: timedelta,
        iterations_completed: int,
        max_iter: int,
        until_complete: bool,
    ) -> tuple[StopReason, bool]:
        """Check timeout and iteration limits."""
        elapsed = datetime.now() - start_time
        if elapsed > timeout_delta:
            timeout_mins = int(timeout_delta.total_seconds() / 60)
            self.output.warning(f"Timeout reached ({timeout_mins} minutes)")
            return StopReason.TIMEOUT, True

        if not until_complete and iterations_completed >= max_iter:
            self.output.warning(f"Max iterations reached ({max_iter})")
            return StopReason.MAX_ITERATIONS, True

        return StopReason.COMPLETE, False

    def _check_prd_completion(self, prd: PrdDocument) -> tuple[StopReason, bool]:
        """Check if all stories are complete."""
        if all_stories_complete(prd):
            self.output.success("All stories have passes: true")
            return StopReason.COMPLETE, True

        pending = get_pending_stories(prd)
        if not pending:
            self.output.warning("No pending stories")
            return StopReason.NO_TASKS, True

        return StopReason.COMPLETE, False

    def _check_limits(
        self,
        max_iter: int,
        until_complete: bool,
        prd: PrdDocument,
    ) -> tuple[StopReason, bool]:
        """Check progress limits."""
        can_continue, signal = check_limits(
            max_iterations=max_iter if not until_complete else 999999,
            max_failures=self.config.limits.max_task_failures,
            total_tasks=len(prd.user_stories),
        )

        if not can_continue:
            if signal and "COMPLETE" in signal:
                self.output.success(signal)
                return StopReason.COMPLETE, True
            else:
                self.output.success(signal or "Limit reached")
                return StopReason.MAX_ITERATIONS, True

        return StopReason.COMPLETE, False

    def _handle_iteration_result(self, result: IterationResult) -> tuple[StopReason, bool]:
        """Handle the result of an iteration."""
        if not result.success:
            if result.error == "AFK_COMPLETE":
                self.output.success("All tasks completed!")
                return StopReason.COMPLETE, True
            elif result.error == "AFK_LIMIT_REACHED":
                return StopReason.MAX_ITERATIONS, True
            else:
                self.output.error(f"Iteration failed: {result.error}")
                return StopReason.AI_ERROR, True

        return StopReason.COMPLETE, False

    def _mark_task_in_progress(self, story: UserStory) -> None:
        """Mark a task as in_progress in progress.json."""
        progress = SessionProgress.load()
        progress.set_task_status(
            task_id=story.id,
            status="in_progress",
            source=story.source,
        )

    def _check_story_completion(self, old_prd: PrdDocument) -> int:
        """Check for newly completed stories and update progress."""
        new_prd = load_prd()

        # Find stories that newly passed
        old_passed_ids = {s.id for s in old_prd.user_stories if s.passes}
        newly_completed = [
            s for s in new_prd.user_stories if s.passes and s.id not in old_passed_ids
        ]

        if newly_completed:
            self.output.success(f"✓ {len(newly_completed)} story/stories marked complete")

            # Update progress.json with completed tasks
            progress = SessionProgress.load()
            for story in newly_completed:
                progress.set_task_status(
                    task_id=story.id,
                    status="completed",
                    source=story.source,
                )
                # Show celebration animation for each completed task
                self.output.show_celebration(story.id)

            return len(newly_completed)

        return 0


# =============================================================================
# Quality Gates
# =============================================================================


def run_quality_gates(
    feedback_loops: FeedbackLoopsConfig,
    console: Console | None = None,
    output_handler: OutputHandler | None = None,
    continuing: bool = False,
) -> QualityGateResult:
    """Run all configured quality gates.

    Args:
        feedback_loops: Feedback loop configuration
        console: Optional console for output
        output_handler: Optional OutputHandler for feedback display integration
        continuing: If True and gates fail, show 'Continuing...' indicator

    Returns:
        QualityGateResult with pass/fail status and outputs
    """
    output = output_handler or OutputHandler(console)
    gates: dict[str, str] = {}

    if feedback_loops.types:
        gates["types"] = feedback_loops.types
    if feedback_loops.lint:
        gates["lint"] = feedback_loops.lint
    if feedback_loops.test:
        gates["test"] = feedback_loops.test
    if feedback_loops.build:
        gates["build"] = feedback_loops.build
    gates.update(feedback_loops.custom)

    if not gates:
        return QualityGateResult(passed=True, failed_gates=[], output={})

    failed_gates: list[str] = []
    outputs: dict[str, str] = {}

    for name, cmd in gates.items():
        output.console.print(f"  [dim]Running {name}...[/dim]", end=" ")
        try:
            result = subprocess.run(
                cmd,
                shell=True,
                capture_output=True,
                text=True,
                timeout=300,
            )
            outputs[name] = result.stdout + result.stderr

            if result.returncode != 0:
                failed_gates.append(name)
                output.console.print("[red]✗[/red]")
            else:
                output.console.print("[green]✓[/green]")

        except subprocess.TimeoutExpired:
            failed_gates.append(name)
            outputs[name] = "Timed out after 5 minutes"
            output.console.print("[red]timeout[/red]")
        except Exception as e:
            failed_gates.append(name)
            outputs[name] = str(e)
            output.console.print("[red]error[/red]")

    # Show visual feedback for gate results
    if failed_gates:
        output.show_gates_failed(failed_gates, continuing=continuing)
    elif gates:
        # All gates passed - show success feedback
        output.show_gates_passed(list(gates.keys()))

    return QualityGateResult(
        passed=len(failed_gates) == 0,
        failed_gates=failed_gates,
        output=outputs,
    )


# =============================================================================
# Prompt-only mode (ralf.sh style)
# =============================================================================


def run_loop(
    config: AfkConfig,
    max_iterations: int | None = None,
    branch: str | None = None,
    until_complete: bool = False,
    timeout_override: int | None = None,
    on_iteration_complete: Callable[[int, IterationResult], None] | None = None,
    resume: bool = False,
    feedback_mode: str | None = None,
    show_mascot: bool | None = None,
) -> RunResult:
    """Run the autonomous afk loop.

    Convenience function that creates a LoopController and runs it.

    Args:
        config: afk configuration
        max_iterations: Override for max iterations
        branch: Branch name to create/checkout
        until_complete: If True, run until all tasks done
        timeout_override: Override timeout in minutes
        on_iteration_complete: Callback after each iteration
        resume: If True, continue from last session
        feedback_mode: Feedback display mode ('full', 'minimal', 'off')
        show_mascot: Whether to display ASCII mascot (default: from config)

    Returns:
        RunResult with session statistics
    """
    return LoopController(config).run(
        max_iterations=max_iterations,
        branch=branch,
        until_complete=until_complete,
        timeout_override=timeout_override,
        on_iteration_complete=on_iteration_complete,
        resume=resume,
        feedback_mode=feedback_mode,
        show_mascot=show_mascot,
    )


def run_iteration(
    config: AfkConfig,
    iteration: int,
    on_output: Callable[[str], None] | None = None,
    stream: bool = True,
) -> IterationResult:
    """Run a single iteration with fresh AI context.

    Convenience function that creates an IterationRunner and runs it.

    Args:
        config: afk configuration
        iteration: Current iteration number
        on_output: Optional callback for streaming output
        stream: If True, stream output in real-time

    Returns:
        IterationResult with success status and any output
    """
    runner = IterationRunner(config)
    return runner.run(iteration, on_output=on_output, stream=stream)


def _contains_completion_signal(text: str | None) -> bool:
    """Check if text contains any completion signal."""
    if not text:
        return False
    return any(signal in text for signal in COMPLETION_SIGNALS)

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
from afk.feedback import MetricsCollector
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
    ) -> None:
        """Initialise output handler.

        Args:
            console: Rich Console instance (creates default if None)
            completion_signals: Signals to detect for early termination
        """
        self.console = console or Console()
        self.signals = completion_signals or COMPLETION_SIGNALS
        self._parser = OutputParser()
        self._collector = MetricsCollector()

    @property
    def metrics_collector(self) -> MetricsCollector:
        """Access the metrics collector for reading accumulated metrics."""
        return self._collector

    @property
    def output_parser(self) -> OutputParser:
        """Access the output parser for external use if needed."""
        return self._parser

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
        MetricsCollector.

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
            elif isinstance(event, ErrorEvent):
                self._collector.metrics.errors.append(event.error_message)
                self._collector.metrics.last_activity = datetime.now()
            elif isinstance(event, WarningEvent):
                self._collector.metrics.warnings.append(event.warning_message)
                self._collector.metrics.last_activity = datetime.now()

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

    def prompt_only_panel(
        self,
        prompt_name: str,
        ai_cli: str,
        max_iterations: int,
    ) -> None:
        """Display prompt-only mode panel."""
        self.console.print(
            Panel.fit(
                f"[bold]Prompt-only mode[/bold]\n\n"
                f"Prompt: [cyan]{prompt_name}[/cyan]\n"
                f"AI CLI: [cyan]{ai_cli}[/cyan]\n"
                f"Max iterations: [cyan]{max_iterations}[/cyan]",
                title="afk",
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

    def session_complete_panel_simple(
        self,
        iterations: int,
        duration: float,
        stop_reason: StopReason,
    ) -> None:
        """Display simple session complete panel (for prompt-only mode)."""
        self.console.print(
            Panel.fit(
                f"[bold]Session Complete[/bold]\n\n"
                f"Iterations: [cyan]{iterations}[/cyan]\n"
                f"Duration: [cyan]{duration:.1f}s[/cyan]\n"
                f"Reason: [cyan]{stop_reason.value}[/cyan]",
                title="afk",
            )
        )


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

    def run_with_static_prompt(
        self,
        iteration: int,
        max_iterations: int,
        prompt_content: str,
    ) -> tuple[IterationResult, bool]:
        """Run iteration with static prompt content (prompt-only mode).

        Args:
            iteration: Current iteration number
            max_iterations: Total iterations for display
            prompt_content: Static prompt content

        Returns:
            Tuple of (IterationResult, completion_detected)
        """
        cmd = [self.config.ai_cli.command] + self.config.ai_cli.args

        self.output.iteration_header(iteration, max_iterations)

        return self._execute_with_completion_detection(cmd, prompt_content)

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

    def _execute_with_completion_detection(
        self,
        cmd: list[str],
        prompt_content: str,
    ) -> tuple[IterationResult, bool]:
        """Execute command and detect completion signal."""
        try:
            # Pass prompt as final argument (universal across AI CLIs)
            full_cmd = cmd + [prompt_content]

            process = subprocess.Popen(
                full_cmd,
                stdin=subprocess.DEVNULL,
                stdout=subprocess.PIPE,
                stderr=subprocess.STDOUT,
                text=True,
                bufsize=1,
            )

            output_buffer: list[str] = []
            completion_detected = False

            if process.stdout:
                for line in iter(process.stdout.readline, ""):
                    self.output.stream_line(line)
                    output_buffer.append(line)

                    if self.output.contains_completion_signal(line):
                        self.output.completion_detected()
                        completion_detected = True
                        process.terminate()
                        break

            if completion_detected:
                return IterationResult(success=True, output="".join(output_buffer)), True

            process.wait()

            if process.returncode != 0:
                return (
                    IterationResult(
                        success=False,
                        error=f"AI CLI exited with code {process.returncode}",
                        output="".join(output_buffer),
                    ),
                    False,
                )

            return IterationResult(success=True, output="".join(output_buffer)), False

        except subprocess.TimeoutExpired:
            process.kill()
            return IterationResult(success=False, error="Iteration timed out"), False
        except FileNotFoundError:
            return (
                IterationResult(
                    success=False,
                    error=f"AI CLI not found: {self.config.ai_cli.command}",
                ),
                False,
            )


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
    ) -> RunResult:
        """Run the autonomous loop.

        Args:
            max_iterations: Override for max iterations
            branch: Branch name to create/checkout
            until_complete: If True, run until all tasks done
            timeout_override: Override timeout in minutes
            on_iteration_complete: Callback after each iteration
            resume: If True, continue from last session

        Returns:
            RunResult with session statistics
        """
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
            total_count=len(prd.userStories),
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
        prd: object,  # PRD type
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
            total_tasks=len(prd.userStories),
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
        old_passed_ids = {s.id for s in old_prd.userStories if s.passes}
        newly_completed = [
            s for s in new_prd.userStories if s.passes and s.id not in old_passed_ids
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

            return len(newly_completed)

        return 0


# =============================================================================
# Quality Gates
# =============================================================================


def run_quality_gates(
    feedback_loops: FeedbackLoopsConfig,
    console: Console | None = None,
) -> QualityGateResult:
    """Run all configured quality gates.

    Args:
        feedback_loops: Feedback loop configuration
        console: Optional console for output

    Returns:
        QualityGateResult with pass/fail status and outputs
    """
    output = OutputHandler(console)
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

    return QualityGateResult(
        passed=len(failed_gates) == 0,
        failed_gates=failed_gates,
        output=outputs,
    )


# =============================================================================
# Backwards-compatible module-level functions
# =============================================================================

# Default console for module-level functions
console = Console()


def run_iteration(
    config: AfkConfig,
    iteration: int,
    on_output: Callable[[str], None] | None = None,
    stream: bool = True,
) -> IterationResult:
    """Run a single iteration with fresh AI context.

    This is a backwards-compatible wrapper around IterationRunner.

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


def run_loop(
    config: AfkConfig,
    max_iterations: int | None = None,
    branch: str | None = None,
    until_complete: bool = False,
    timeout_override: int | None = None,
    on_iteration_complete: Callable[[int, IterationResult], None] | None = None,
    resume: bool = False,
) -> RunResult:
    """Run the autonomous afk loop.

    This is a backwards-compatible wrapper around LoopController.

    Args:
        config: afk configuration
        max_iterations: Override for max iterations
        branch: Branch name to create/checkout
        until_complete: If True, run until all tasks done
        timeout_override: Override timeout in minutes
        on_iteration_complete: Callback after each iteration
        resume: If True, continue from last session

    Returns:
        RunResult with session statistics
    """
    controller = LoopController(config)
    return controller.run(
        max_iterations=max_iterations,
        branch=branch,
        until_complete=until_complete,
        timeout_override=timeout_override,
        on_iteration_complete=on_iteration_complete,
        resume=resume,
    )


def run_prompt_only(
    prompt_file: Path,
    config: AfkConfig,
    max_iterations: int = 10,
    timeout_minutes: int | None = None,
) -> RunResult:
    """Run in prompt-only mode (ralf.sh style).

    This is a simpler mode that just pipes a static prompt to the AI CLI
    each iteration, checking for completion signals in the output.

    Args:
        prompt_file: Path to the prompt file
        config: afk configuration
        max_iterations: Maximum number of iterations
        timeout_minutes: Override timeout in minutes

    Returns:
        RunResult with session statistics
    """
    output = OutputHandler()
    runner = IterationRunner(config, output)

    start_time = datetime.now()
    timeout = timeout_minutes or config.limits.timeout_minutes
    timeout_delta = timedelta(minutes=timeout)

    prompt_content = prompt_file.read_text()

    output.prompt_only_panel(
        prompt_name=prompt_file.name,
        ai_cli=config.ai_cli.command,
        max_iterations=max_iterations,
    )

    iterations_completed = 0
    stop_reason = StopReason.COMPLETE

    try:
        for iteration in range(1, max_iterations + 1):
            elapsed = datetime.now() - start_time
            if elapsed > timeout_delta:
                stop_reason = StopReason.TIMEOUT
                output.warning(f"Timeout reached ({timeout} minutes)")
                break

            result, completion_detected = runner.run_with_static_prompt(
                iteration, max_iterations, prompt_content
            )

            if completion_detected:
                stop_reason = StopReason.COMPLETE
                iterations_completed = iteration
                break

            if not result.success:
                output.error(result.error or "Unknown error")
                stop_reason = StopReason.AI_ERROR
                break

            iterations_completed = iteration
            time.sleep(1)

        else:
            stop_reason = StopReason.MAX_ITERATIONS
            output.warning(f"Max iterations reached ({max_iterations})")

    except KeyboardInterrupt:
        stop_reason = StopReason.USER_INTERRUPT
        output.warning("Interrupted by user")

    duration = (datetime.now() - start_time).total_seconds()

    output.session_complete_panel_simple(
        iterations=iterations_completed,
        duration=duration,
        stop_reason=stop_reason,
    )

    return RunResult(
        iterations_completed=iterations_completed,
        tasks_completed=0,
        stop_reason=stop_reason,
        duration_seconds=duration,
        archived_to=None,
    )


def _contains_completion_signal(output: str | None) -> bool:
    """Check if output contains any completion signal.

    Backwards-compatible function wrapping OutputHandler method.
    """
    handler = OutputHandler()
    return handler.contains_completion_signal(output)

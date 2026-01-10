"""Runner for autonomous afk loops.

This module implements the Ralph Wiggum pattern: spawning fresh AI CLI
instances for each iteration, ensuring clean context between runs.
"""

from __future__ import annotations

import subprocess
import time
from collections.abc import Callable
from dataclasses import dataclass
from datetime import datetime, timedelta
from enum import Enum
from pathlib import Path

from rich.console import Console
from rich.panel import Panel

from afk.config import AfkConfig, FeedbackLoopsConfig
from afk.git_ops import archive_session, auto_commit, clear_session, create_branch
from afk.progress import SessionProgress, check_limits
from afk.prompt import generate_prompt
from afk.sources import aggregate_tasks

console = Console()


@dataclass
class QualityGateResult:
    """Result of running quality gates."""

    passed: bool
    failed_gates: list[str]
    output: dict[str, str]


def run_quality_gates(feedback_loops: FeedbackLoopsConfig) -> QualityGateResult:
    """Run all configured quality gates.

    Args:
        feedback_loops: Feedback loop configuration

    Returns:
        QualityGateResult with pass/fail status and outputs
    """
    gates: dict[str, str] = {}

    # Collect all configured gates
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
        console.print(f"  [dim]Running {name}...[/dim]", end=" ")
        try:
            result = subprocess.run(
                cmd,
                shell=True,
                capture_output=True,
                text=True,
                timeout=300,  # 5 minute timeout per gate
            )
            outputs[name] = result.stdout + result.stderr

            if result.returncode != 0:
                failed_gates.append(name)
                console.print("[red]✗[/red]")
            else:
                console.print("[green]✓[/green]")

        except subprocess.TimeoutExpired:
            failed_gates.append(name)
            outputs[name] = "Timed out after 5 minutes"
            console.print("[red]timeout[/red]")
        except Exception as e:
            failed_gates.append(name)
            outputs[name] = str(e)
            console.print("[red]error[/red]")

    return QualityGateResult(
        passed=len(failed_gates) == 0,
        failed_gates=failed_gates,
        output=outputs,
    )


class StopReason(Enum):
    """Reasons for stopping the runner."""

    COMPLETE = "All tasks completed"
    MAX_ITERATIONS = "Maximum iterations reached"
    TIMEOUT = "Session timeout reached"
    NO_TASKS = "No tasks available"
    USER_INTERRUPT = "User interrupted"
    AI_ERROR = "AI CLI error"


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


def run_iteration(
    config: AfkConfig,
    iteration: int,
    on_output: Callable[[str], None] | None = None,
) -> IterationResult:
    """Run a single iteration with fresh AI context.

    This spawns a new AI CLI process, passing the generated prompt,
    and waits for it to complete. Each iteration gets clean context.

    Args:
        config: afk configuration
        iteration: Current iteration number
        on_output: Optional callback for streaming output

    Returns:
        IterationResult with success status and any output
    """
    # Generate prompt for this iteration (bootstrap mode for autonomous loop)
    prompt = generate_prompt(config, bootstrap=True)

    # Check for stop signals in prompt
    if "AFK_COMPLETE" in prompt:
        return IterationResult(
            success=True,
            error="AFK_COMPLETE",
        )
    if "AFK_LIMIT_REACHED" in prompt:
        return IterationResult(
            success=False,
            error="AFK_LIMIT_REACHED",
        )

    # Build command
    cmd = [config.ai_cli.command] + config.ai_cli.args

    console.print(f"\n[cyan]━━━ Iteration {iteration} ━━━[/cyan]")
    console.print(f"[dim]Running: {' '.join(cmd)}[/dim]")

    try:
        # Spawn fresh AI process with prompt on stdin
        process = subprocess.Popen(
            cmd,
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
        )

        # Send prompt and get output
        stdout, _ = process.communicate(input=prompt, timeout=config.limits.timeout_minutes * 60)

        if on_output and stdout:
            on_output(stdout)

        if process.returncode != 0:
            return IterationResult(
                success=False,
                error=f"AI CLI exited with code {process.returncode}",
                output=stdout or "",
            )

        return IterationResult(
            success=True,
            output=stdout or "",
        )

    except subprocess.TimeoutExpired:
        process.kill()
        return IterationResult(
            success=False,
            error="Iteration timed out",
        )
    except FileNotFoundError:
        return IterationResult(
            success=False,
            error=f"AI CLI not found: {config.ai_cli.command}",
        )
    except Exception as e:
        return IterationResult(
            success=False,
            error=str(e),
        )


def run_loop(
    config: AfkConfig,
    max_iterations: int | None = None,
    branch: str | None = None,
    until_complete: bool = False,
    timeout_override: int | None = None,
    on_iteration_complete: Callable[[int, IterationResult], None] | None = None,
) -> RunResult:
    """Run the autonomous afk loop.

    This is the core Ralph Wiggum pattern: repeatedly spawn fresh AI
    instances until all tasks complete or limits are reached.

    Args:
        config: afk configuration
        max_iterations: Override for max iterations (None uses config)
        branch: Branch name to create/checkout (None skips branching)
        until_complete: If True, ignore max_iterations and run until done
        timeout_override: Override timeout in minutes
        on_iteration_complete: Callback after each iteration

    Returns:
        RunResult with session statistics
    """
    start_time = datetime.now()
    timeout_minutes = timeout_override or config.limits.timeout_minutes
    timeout_delta = timedelta(minutes=timeout_minutes)
    max_iter = max_iterations or config.limits.max_iterations

    # Handle branching
    if branch and config.git.auto_branch:
        full_branch = f"{config.git.branch_prefix}{branch}"
        console.print(f"[cyan]Creating/switching to branch:[/cyan] {full_branch}")
        create_branch(branch, config)

    # Archive previous session if starting fresh
    if config.archive.enabled:
        progress = SessionProgress.load()
        if progress.iterations > 0:
            archive_path = archive_session(config, reason="new_run")
            if archive_path:
                console.print(f"[dim]Archived previous session to: {archive_path}[/dim]")
            clear_session()

    console.print(
        Panel.fit(
            f"[bold]Starting afk loop[/bold]\n\n"
            f"AI CLI: [cyan]{config.ai_cli.command}[/cyan]\n"
            f"Max iterations: [cyan]{max_iter if not until_complete else '∞'}[/cyan]\n"
            f"Timeout: [cyan]{timeout_minutes} minutes[/cyan]\n"
            f"Tasks: [cyan]{len(aggregate_tasks(config.sources))}[/cyan]",
            title="afk run",
        )
    )

    iterations_completed = 0
    tasks_completed_this_session = 0
    stop_reason = StopReason.COMPLETE
    archived_to = None

    try:
        while True:
            # Check timeout
            elapsed = datetime.now() - start_time
            if elapsed > timeout_delta:
                stop_reason = StopReason.TIMEOUT
                console.print(f"\n[yellow]Timeout reached ({timeout_minutes} minutes)[/yellow]")
                break

            # Check iteration limit (unless until_complete)
            if not until_complete and iterations_completed >= max_iter:
                stop_reason = StopReason.MAX_ITERATIONS
                console.print(f"\n[yellow]Max iterations reached ({max_iter})[/yellow]")
                break

            # Check if all tasks complete
            tasks = aggregate_tasks(config.sources)
            if not tasks:
                stop_reason = StopReason.NO_TASKS
                console.print("\n[yellow]No tasks available[/yellow]")
                break

            progress = SessionProgress.load()
            can_continue, signal = check_limits(
                max_iterations=max_iter if not until_complete else 999999,
                max_failures=config.limits.max_task_failures,
                total_tasks=len(tasks),
            )

            if not can_continue:
                if signal and "COMPLETE" in signal:
                    stop_reason = StopReason.COMPLETE
                else:
                    stop_reason = StopReason.MAX_ITERATIONS
                console.print(f"\n[green]{signal}[/green]")
                break

            # Run iteration with fresh context
            iteration_num = iterations_completed + 1
            result = run_iteration(config, iteration_num)

            if on_iteration_complete:
                on_iteration_complete(iteration_num, result)

            if not result.success:
                if result.error == "AFK_COMPLETE":
                    stop_reason = StopReason.COMPLETE
                    console.print("\n[green]All tasks completed![/green]")
                    break
                elif result.error == "AFK_LIMIT_REACHED":
                    stop_reason = StopReason.MAX_ITERATIONS
                    break
                else:
                    console.print(f"\n[red]Iteration failed:[/red] {result.error}")
                    stop_reason = StopReason.AI_ERROR
                    break

            iterations_completed += 1

            # Check for newly completed tasks
            new_progress = SessionProgress.load()
            newly_completed = [
                t
                for t in new_progress.get_completed_tasks()
                if t.completed_at and t.completed_at > start_time.isoformat()
            ]

            for task in newly_completed:
                # Run quality gates before committing
                console.print(f"\n[cyan]Quality gates for {task.id}:[/cyan]")
                gate_result = run_quality_gates(config.feedback_loops)

                if not gate_result.passed:
                    failed = ", ".join(gate_result.failed_gates)
                    console.print(f"[yellow]⚠ Quality gates failed: {failed}[/yellow]")
                    console.print("[dim]Skipping auto-commit. Fix and commit manually.[/dim]")
                else:
                    if config.git.auto_commit:
                        success = auto_commit(
                            task.id,
                            task.message or "Task completed",
                            config,
                        )
                        if success:
                            console.print(f"[green]✓ Committed:[/green] {task.id}")

                tasks_completed_this_session += 1

            # Brief pause between iterations
            time.sleep(1)

    except KeyboardInterrupt:
        stop_reason = StopReason.USER_INTERRUPT
        console.print("\n[yellow]Interrupted by user[/yellow]")

    # Archive final session
    if config.archive.enabled:
        archived_to = archive_session(config, reason=stop_reason.name.lower())

    duration = (datetime.now() - start_time).total_seconds()

    # Summary
    console.print(
        Panel.fit(
            f"[bold]Session Complete[/bold]\n\n"
            f"Iterations: [cyan]{iterations_completed}[/cyan]\n"
            f"Tasks completed: [cyan]{tasks_completed_this_session}[/cyan]\n"
            f"Duration: [cyan]{duration:.1f}s[/cyan]\n"
            f"Reason: [cyan]{stop_reason.value}[/cyan]"
            + (f"\nArchived to: [dim]{archived_to}[/dim]" if archived_to else ""),
            title="afk",
        )
    )

    return RunResult(
        iterations_completed=iterations_completed,
        tasks_completed=tasks_completed_this_session,
        stop_reason=stop_reason,
        duration_seconds=duration,
        archived_to=archived_to,
    )

"""CLI interface for afk."""

from __future__ import annotations

from pathlib import Path

import click
from rich.console import Console
from rich.panel import Panel
from rich.table import Table

from afk import __version__
from afk.config import AFK_DIR, CONFIG_FILE, AfkConfig

console = Console()


@click.group(invoke_without_command=True)
@click.version_option(version=__version__, prog_name="afk")
@click.pass_context
def main(ctx: click.Context) -> None:
    """afk - Autonomous AI coding loops.

    Run AI coding tasks in a loop, Ralph Wiggum style.

    \b
    Zero-config usage:
      afk go                 # Auto-detect, run 10 iterations
      afk go 20              # Auto-detect, run 20 iterations
      afk go TODO.md 5       # Use TODO.md as source, run 5 iterations

    \b
    Full control:
      afk run 10             # Explicit run command
      afk run --until-complete
      afk init               # Explicit initialization
    """
    ctx.ensure_object(dict)
    ctx.obj["config"] = AfkConfig.load()

    # If no subcommand, show help
    if ctx.invoked_subcommand is None:
        click.echo(ctx.get_help())


@main.command("go")
@click.argument("iterations_or_source", required=False)
@click.argument("iterations_if_source", type=int, required=False)
@click.option("--dry-run", "-n", is_flag=True, help="Show what would run without running")
@click.option("--until-complete", "-u", is_flag=True, help="Run until all tasks complete")
@click.option(
    "--feedback",
    type=click.Choice(["full", "minimal", "off"]),
    default=None,
    help="Feedback display mode (default: from config or 'full')",
)
@click.pass_context
def go_command(
    ctx: click.Context,
    iterations_or_source: str | None,
    iterations_if_source: int | None,
    dry_run: bool,
    until_complete: bool,
    feedback: str | None,
) -> None:
    """Quick start with zero config.

    \b
    Examples:
      afk go                 # Auto-detect, run 10 iterations
      afk go 20              # Auto-detect, run 20 iterations
      afk go -u              # Run until all tasks complete
      afk go TODO.md 5       # Use TODO.md as source, run 5 iterations
      afk go --feedback off  # Run without feedback display
    """
    _run_zero_config(
        ctx, iterations_or_source, iterations_if_source, dry_run, until_complete, feedback
    )


def _run_zero_config(
    ctx: click.Context,
    iterations_or_source: str | None,
    iterations_if_source: int | None,
    dry_run: bool,
    until_complete: bool = False,
    feedback_mode: str | None = None,
) -> None:
    """Handle zero-config invocation (afk go, afk go 10, afk go TODO.md 5)."""
    from afk.bootstrap import (
        detect_prompt_file,
        ensure_ai_cli_configured,
        infer_config,
        infer_sources,
    )
    from afk.runner import LoopController

    # Parse arguments
    iterations = 10  # default
    explicit_source: str | None = None

    if iterations_or_source is not None:
        # Check if it's a number or a file path
        try:
            iterations = int(iterations_or_source)
        except ValueError:
            # It's a source file
            explicit_source = iterations_or_source
            iterations = iterations_if_source or 10

    # Get or infer config
    config = infer_config()

    # Ensure AI CLI is configured (first-run experience)
    ai_cli = ensure_ai_cli_configured(config, console)
    config.ai_cli = ai_cli

    # Handle explicit source
    if explicit_source:
        from afk.config import SourceConfig

        path = Path(explicit_source)
        if not path.exists():
            console.print(f"[red]Source file not found:[/red] {explicit_source}")
            ctx.exit(1)

        # Determine source type
        if path.suffix == ".json":
            source = SourceConfig(type="json", path=explicit_source)
        elif path.suffix == ".md":
            source = SourceConfig(type="markdown", path=explicit_source)
        else:
            console.print(f"[red]Unknown source type:[/red] {explicit_source}")
            ctx.exit(1)

        config.sources = [source]

    # Infer sources if none configured
    # But if .afk/prd.json exists with stories, use it directly - it's the source of truth
    # (created by afk prd parse or placed there manually)
    if not config.sources:
        from afk.prd_store import load_prd

        existing_prd = load_prd()
        if not existing_prd.user_stories:
            # No PRD exists, try to infer sources
            inferred = infer_sources()
            if inferred:
                config.sources = inferred

    # Check if we have work to do (either sources or existing PRD)
    if not config.sources:
        from afk.prd_store import load_prd

        existing_prd = load_prd()
        if not existing_prd.user_stories:
            # No sources AND no PRD - check for prompt file (ralf.sh mode)
            prompt_file = detect_prompt_file()
            if prompt_file:
                from afk.runner import run_prompt_only

                if dry_run:
                    console.print(Panel.fit("[bold]Dry Run (Prompt-only)[/bold]", title="afk"))
                    console.print()
                    console.print(f"  [cyan]Prompt file:[/cyan] {prompt_file.name}")
                    console.print(f"  [cyan]Iterations:[/cyan] {iterations}")
                    console.print(f"  [cyan]AI CLI:[/cyan] {config.ai_cli.command}")
                    return

                run_prompt_only(
                    prompt_file=prompt_file,
                    config=config,
                    max_iterations=iterations,
                )
                return
            else:
                console.print("[red]No task sources found.[/red]")
                console.print()
                console.print("To get started, either:")
                console.print()
                console.print("  [bold]Parse a PRD:[/bold]")
                console.print("    [cyan]afk prd parse requirements.md[/cyan]")
                console.print("    [dim]Creates .afk/prd.json from your requirements doc[/dim]")
                console.print()
                console.print("  [bold]Or create a task file:[/bold]")
                console.print("    • [cyan]TODO.md[/cyan] - Markdown checklist")
                console.print("    • [cyan]tasks.json[/cyan] - JSON task file")
                console.print("    • [cyan]prompt.md[/cyan] - Single prompt (ralf.sh style)")
                console.print("    • [cyan].beads/[/cyan] - Beads issue tracker")
                ctx.exit(1)

    if dry_run:
        console.print(Panel.fit("[bold]Dry Run[/bold]", title="afk"))
        console.print()
        console.print(f"  [cyan]Iterations:[/cyan] {iterations}")
        console.print(f"  [cyan]AI CLI:[/cyan] {config.ai_cli.command}")
        console.print()
        console.print("  [cyan]Sources:[/cyan]")
        for src in config.sources:
            path_info = f" ({src.path})" if src.path else ""
            console.print(f"    • {src.type}{path_info}")
        return

    # Determine feedback mode: CLI flag > config > default
    effective_feedback = feedback_mode or config.feedback.mode

    # Run the loop
    LoopController(config).run(
        max_iterations=iterations, until_complete=until_complete, feedback_mode=effective_feedback
    )


@main.command()
@click.option("--dry-run", "-n", is_flag=True, help="Show what would be configured without writing")
@click.option("--force", "-f", is_flag=True, help="Overwrite existing config")
@click.option("--yes", "-y", is_flag=True, help="Accept all defaults without prompting")
def init(dry_run: bool, force: bool, yes: bool) -> None:
    """Initialize afk by analysing the project.

    Detects project type, available tools, task sources, and context files
    to generate a sensible configuration.
    """
    from afk.bootstrap import analyse_project, generate_config

    # Check for existing config
    if AFK_DIR.exists() and not force and not dry_run:
        console.print(
            "[yellow]afk already initialized.[/yellow] "
            "Use --force to reconfigure or --dry-run to preview."
        )
        return

    result = analyse_project()

    # Display analysis
    console.print(Panel.fit("[bold]Project Analysis[/bold]", title="afk init"))
    console.print()

    # Stack detection
    if result.stack:
        console.print(f"  [green]✓[/green] Stack: [cyan]{result.stack.name}[/cyan]")
    else:
        console.print("  [yellow]?[/yellow] Stack: [dim]Not detected[/dim]")

    # Available tools
    console.print()
    console.print("  [bold]Tools:[/bold]")
    for tool, available in result.available_tools.items():
        icon = "[green]✓[/green]" if available else "[dim]✗[/dim]"
        console.print(f"    {icon} {tool}")

    # Sources
    console.print()
    console.print("  [bold]Task Sources:[/bold]")
    if result.sources:
        for src in result.sources:
            path_info = f" ({src.path})" if src.path else ""
            console.print(f"    [green]✓[/green] {src.type}{path_info}")
    else:
        console.print("    [dim]None detected[/dim]")

    # Context files
    console.print()
    console.print("  [bold]Context Files:[/bold]")
    if result.context_files:
        for f in result.context_files:
            console.print(f"    [green]✓[/green] {f}")
    else:
        console.print("    [dim]None found[/dim]")

    # Feedback loops
    if result.stack:
        console.print()
        console.print("  [bold]Feedback Loops:[/bold]")
        loops = result.stack.feedback_loops
        if loops.lint:
            console.print(f"    lint: [cyan]{loops.lint}[/cyan]")
        if loops.types:
            console.print(f"    types: [cyan]{loops.types}[/cyan]")
        if loops.test:
            console.print(f"    test: [cyan]{loops.test}[/cyan]")
        if loops.build:
            console.print(f"    build: [cyan]{loops.build}[/cyan]")

    # Warnings
    if result.warnings:
        console.print()
        for warning in result.warnings:
            console.print(f"  [yellow]⚠[/yellow] {warning}")

    if dry_run:
        console.print()
        console.print("[dim]Dry run - no changes made.[/dim]")
        return

    # Confirm unless --yes
    if not yes:
        console.print()
        if not click.confirm("Apply this configuration?", default=True):
            console.print("[dim]Cancelled.[/dim]")
            return

    # Generate and save config
    config = generate_config(result)
    AFK_DIR.mkdir(parents=True, exist_ok=True)
    config.save()

    console.print()
    console.print(
        Panel.fit(
            "[green]Configuration saved![/green]\n\n"
            f"Config: [cyan]{CONFIG_FILE}[/cyan]\n\n"
            "Next steps:\n"
            "  1. Review config: [cyan]cat .afk/config.json[/cyan]\n"
            "  2. Check status: [cyan]afk status[/cyan]\n"
            "  3. Get next prompt: [cyan]afk next[/cyan]",
            title="afk",
        )
    )


@main.command()
@click.pass_context
def status(ctx: click.Context) -> None:
    """Show current status and tasks."""
    config: AfkConfig = ctx.obj["config"]

    if not AFK_DIR.exists():
        console.print("[red]afk not initialized.[/red] Run [cyan]afk init[/cyan] first.")
        return

    # Sources table
    sources_table = Table(title="Task Sources", show_header=True)
    sources_table.add_column("Type", style="cyan")
    sources_table.add_column("Path/Config", style="white")

    if config.sources:
        for source in config.sources:
            path = source.path or source.repo or "(default)"
            sources_table.add_row(source.type, path)
    else:
        sources_table.add_row("[dim]none configured[/dim]", "")

    console.print(sources_table)
    console.print()

    # Limits
    limits_table = Table(title="Limits", show_header=True)
    limits_table.add_column("Setting", style="cyan")
    limits_table.add_column("Value", style="white")
    limits_table.add_row("Max iterations", str(config.limits.max_iterations))
    limits_table.add_row("Max task failures", str(config.limits.max_task_failures))
    limits_table.add_row("Timeout", f"{config.limits.timeout_minutes} minutes")

    console.print(limits_table)
    console.print()

    # Git config
    git_table = Table(title="Git Integration", show_header=True)
    git_table.add_column("Setting", style="cyan")
    git_table.add_column("Value", style="white")
    commit_icon = "[green]✓[/green]" if config.git.auto_commit else "[dim]✗[/dim]"
    branch_icon = "[green]✓[/green]" if config.git.auto_branch else "[dim]✗[/dim]"
    git_table.add_row("Auto-commit", commit_icon)
    git_table.add_row("Auto-branch", branch_icon)
    git_table.add_row("Branch prefix", config.git.branch_prefix)

    console.print(git_table)
    console.print()

    # Archive config
    archive_table = Table(title="Archiving", show_header=True)
    archive_table.add_column("Setting", style="cyan")
    archive_table.add_column("Value", style="white")
    enabled_icon = "[green]✓[/green]" if config.archive.enabled else "[dim]✗[/dim]"
    on_change_icon = "[green]✓[/green]" if config.archive.on_branch_change else "[dim]✗[/dim]"
    archive_table.add_row("Enabled", enabled_icon)
    archive_table.add_row("Directory", config.archive.directory)
    archive_table.add_row("On branch change", on_change_icon)

    console.print(archive_table)


@main.group()
def source() -> None:
    """Manage task sources."""
    pass


@source.command("add")
@click.argument("source_type", type=click.Choice(["beads", "json", "markdown", "github"]))
@click.argument("path", required=False)
@click.pass_context
def source_add(ctx: click.Context, source_type: str, path: str | None) -> None:
    """Add a task source."""
    config: AfkConfig = ctx.obj["config"]

    from afk.config import SourceConfig

    # Validate path exists for file-based sources
    if source_type in ("json", "markdown") and path:
        if not Path(path).exists():
            console.print(f"[red]File not found:[/red] {path}")
            return

    new_source = SourceConfig(type=source_type, path=path)  # type: ignore[arg-type]
    config.sources.append(new_source)
    config.save()

    console.print(f"[green]Added source:[/green] {source_type}" + (f" ({path})" if path else ""))


@source.command("list")
@click.pass_context
def source_list(ctx: click.Context) -> None:
    """List configured task sources."""
    config: AfkConfig = ctx.obj["config"]

    if not config.sources:
        console.print("[dim]No sources configured.[/dim] Use [cyan]afk source add[/cyan]")
        return

    for i, src in enumerate(config.sources, 1):
        path_info = f" ({src.path})" if src.path else ""
        console.print(f"  {i}. [cyan]{src.type}[/cyan]{path_info}")


@source.command("remove")
@click.argument("index", type=int)
@click.pass_context
def source_remove(ctx: click.Context, index: int) -> None:
    """Remove a task source by index (1-based)."""
    config: AfkConfig = ctx.obj["config"]

    if index < 1 or index > len(config.sources):
        console.print(f"[red]Invalid index.[/red] Must be 1-{len(config.sources)}")
        return

    removed = config.sources.pop(index - 1)
    config.save()

    console.print(f"[green]Removed source:[/green] {removed.type}")


@main.group()
def prd() -> None:
    """Manage product requirements documents."""
    pass


@prd.command("parse")
@click.argument("input_file", type=click.Path(exists=True))
@click.option("--output", "-o", default=".afk/prd.json", help="Output JSON path")
@click.option("--copy", "-c", "output_mode", flag_value="clipboard", help="Copy to clipboard")
@click.option("--file", "-f", "output_mode", flag_value="file", help="Write prompt to file")
@click.option("--stdout", "-s", "output_mode", flag_value="stdout", help="Print prompt to stdout")
@click.pass_context
def prd_parse(
    ctx: click.Context,
    input_file: str,
    output: str,
    output_mode: str | None,
) -> None:
    """Parse a PRD into a structured JSON feature list.

    Takes a product requirements document (markdown, text, etc.) and generates
    an AI prompt to convert it into the Anthropic-style JSON format.

    Example:

        afk prd parse requirements.md --copy

        afk prd parse PRD.md -o tasks.json
    """
    config: AfkConfig = ctx.obj["config"]

    from afk.output import output_prompt
    from afk.prd import generate_prd_prompt, load_prd_file

    try:
        prd_content = load_prd_file(input_file)
    except FileNotFoundError as e:
        console.print(f"[red]Error:[/red] {e}")
        return

    prompt = generate_prd_prompt(prd_content, output_path=output)

    # Default to stdout if not specified
    output_mode = output_mode or config.output.default

    output_prompt(prompt, mode=output_mode, config=config)  # type: ignore[arg-type]

    console.print()
    console.print(
        f"[dim]Run the prompt with your AI tool, then add the source:[/dim]\n"
        f"  [cyan]afk source add json {output}[/cyan]"
    )


@prd.command("sync")
@click.option("--branch", "-b", help="Branch name for PRD")
@click.pass_context
def prd_sync(ctx: click.Context, branch: str | None) -> None:
    """Sync PRD from all configured sources.

    Aggregates tasks from beads, JSON, markdown, and GitHub into a unified
    .afk/prd.json file. The AI reads this file directly (Ralph pattern).

    Existing completion status (passes: true) is preserved for matching IDs.

    Example:

        afk prd sync

        afk prd sync --branch feature/new-thing
    """
    from afk.prd_store import sync_prd

    config: AfkConfig = ctx.obj["config"]

    if not config.sources:
        console.print("[red]No sources configured.[/red] Run [cyan]afk source add[/cyan] first.")
        return

    console.print("[cyan]Syncing PRD from sources...[/cyan]")

    prd = sync_prd(config, branch_name=branch)

    pending = sum(1 for s in prd.user_stories if not s.passes)
    complete = sum(1 for s in prd.user_stories if s.passes)

    console.print()
    console.print("[green]PRD synced:[/green] .afk/prd.json")
    console.print(f"  Stories: {len(prd.user_stories)} total")
    console.print(f"  Pending: {pending}")
    console.print(f"  Complete: {complete}")

    if prd.branch_name:
        console.print(f"  Branch: {prd.branch_name}")


@prd.command("show")
@click.option("--pending", "-p", is_flag=True, help="Show only pending stories")
@click.pass_context
def prd_show(ctx: click.Context, pending: bool) -> None:
    """Show the current PRD state.

    Displays user stories from .afk/prd.json with their completion status.
    """
    from afk.prd_store import load_prd

    prd = load_prd()

    if not prd.user_stories:
        console.print("[dim]No stories in PRD.[/dim] Run [cyan]afk prd sync[/cyan] first.")
        return

    table = Table(title=f"PRD: {prd.project or 'Untitled'}", show_header=True)
    table.add_column("ID", style="cyan")
    table.add_column("Pri", style="dim", width=3)
    table.add_column("Title", style="white")
    table.add_column("AC", style="dim", width=3)
    table.add_column("Status", style="white")

    for story in prd.user_stories:
        if pending and story.passes:
            continue

        status = "[green]✓ Pass[/green]" if story.passes else "[dim]Pending[/dim]"
        ac_count = str(len(story.acceptance_criteria))

        table.add_row(
            story.id,
            str(story.priority),
            story.title[:50] + ("..." if len(story.title) > 50 else ""),
            ac_count,
            status,
        )

    console.print(table)

    if prd.branch_name:
        console.print(f"\n[dim]Branch: {prd.branch_name}[/dim]")
    if prd.last_synced:
        console.print(f"[dim]Last synced: {prd.last_synced[:19]}[/dim]")


@main.command()
@click.option("--branch", "-b", help="Branch name for PRD")
@click.pass_context
def sync(ctx: click.Context, branch: str | None) -> None:
    """Sync PRD from all configured sources.

    Aggregates tasks from beads, JSON, markdown, and GitHub into a unified
    .afk/prd.json file that the AI reads directly (Ralph pattern).

    Example:

        afk sync

        afk sync --branch feature/new-thing
    """
    from afk.prd_store import sync_prd

    config: AfkConfig = ctx.obj["config"]

    if not config.sources:
        console.print("[red]No sources configured.[/red] Run [cyan]afk source add[/cyan] first.")
        return

    console.print("[cyan]Syncing PRD from sources...[/cyan]")

    prd = sync_prd(config, branch_name=branch)

    pending = sum(1 for s in prd.user_stories if not s.passes)
    complete = sum(1 for s in prd.user_stories if s.passes)

    console.print()
    console.print("[green]PRD synced:[/green] .afk/prd.json")
    console.print(f"  Stories: {len(prd.user_stories)} total")
    console.print(f"  Pending: {pending}")
    console.print(f"  Complete: {complete}")

    if prd.branch_name:
        console.print(f"  Branch: {prd.branch_name}")


@main.command()
@click.option("--copy", "-c", "output_mode", flag_value="clipboard", help="Copy to clipboard")
@click.option("--file", "-f", "output_mode", flag_value="file", help="Write to file")
@click.option("--stdout", "-s", "output_mode", flag_value="stdout", help="Print to stdout")
@click.option("--bootstrap", "-b", is_flag=True, help="Include afk command instructions for AI")
@click.option("--limit", "-l", type=int, help="Override max iterations")
@click.pass_context
def next(
    ctx: click.Context,
    output_mode: str | None,
    bootstrap: bool,
    limit: int | None,
) -> None:
    """Generate prompt for next iteration."""
    config: AfkConfig = ctx.obj["config"]

    if not AFK_DIR.exists():
        console.print("[red]afk not initialized.[/red] Run [cyan]afk init[/cyan] first.")
        return

    if not config.sources:
        console.print("[red]No sources configured.[/red] Run [cyan]afk source add[/cyan] first.")
        return

    # Use configured default if not specified
    output_mode = output_mode or config.output.default

    # Import here to avoid circular imports
    from afk.output import output_prompt
    from afk.prompt import generate_prompt

    prompt = generate_prompt(config, bootstrap=bootstrap, limit_override=limit)
    output_prompt(prompt, mode=output_mode, config=config)  # type: ignore[arg-type]


@main.command()
@click.argument("task_id")
@click.option("--message", "-m", help="Completion message")
@click.pass_context
def done(ctx: click.Context, task_id: str, message: str | None) -> None:
    """Mark a task as complete."""
    from afk.progress import mark_complete

    success = mark_complete(task_id, message=message)

    if success:
        console.print(f"[green]Task completed:[/green] {task_id}")
    else:
        console.print(f"[red]Task not found:[/red] {task_id}")


@main.command()
@click.option("--verbose", "-v", is_flag=True, help="Show full output from failed gates")
@click.pass_context
def verify(ctx: click.Context, verbose: bool) -> None:
    """Run quality gates and report results.

    Runs all configured feedback loops (types, lint, test, build) and reports
    pass/fail status. Use this before marking a story as complete.

    Exit code 0 if all gates pass, 1 if any fail.

    Examples:

        afk verify                # Run all quality gates

        afk verify --verbose      # Show full output from failures
    """
    from afk.runner import run_quality_gates

    config: AfkConfig = ctx.obj["config"]

    # Check if any gates are configured
    fl = config.feedback_loops
    has_gates = fl.types or fl.lint or fl.test or fl.build or fl.custom

    if not has_gates:
        console.print("[yellow]No quality gates configured.[/yellow]")
        console.print("[dim]Add feedback_loops to .afk/config.json[/dim]")
        ctx.exit(0)

    console.print("[cyan]Running quality gates...[/cyan]")
    result = run_quality_gates(config.feedback_loops, console)

    if result.passed:
        console.print()
        console.print("[green]✓ All quality gates passed![/green]")
        console.print("[dim]You can now mark the story as complete (set passes: true)[/dim]")
        ctx.exit(0)
    else:
        console.print()
        console.print(f"[red]✗ Quality gates failed: {', '.join(result.failed_gates)}[/red]")

        if verbose:
            for gate_name in result.failed_gates:
                output = result.output.get(gate_name, "")
                if output:
                    console.print()
                    console.print(f"[bold]{gate_name} output:[/bold]")
                    # Limit output to avoid overwhelming the AI
                    lines = output.strip().split("\n")
                    if len(lines) > 50:
                        console.print("\n".join(lines[:25]))
                        console.print(f"[dim]... ({len(lines) - 50} lines omitted) ...[/dim]")
                        console.print("\n".join(lines[-25:]))
                    else:
                        console.print(output)
        else:
            console.print("[dim]Run with --verbose to see failure details[/dim]")

        ctx.exit(1)


@main.command()
@click.argument("task_id")
@click.option("--message", "-m", help="Failure reason")
@click.pass_context
def fail(ctx: click.Context, task_id: str, message: str | None) -> None:
    """Mark a task as failed."""
    from afk.progress import mark_failed

    failure_count = mark_failed(task_id, message=message)
    console.print(f"[red]Task failed:[/red] {task_id} (attempt {failure_count})")


@main.command()
@click.argument("task_id")
@click.pass_context
def reset(ctx: click.Context, task_id: str) -> None:
    """Reset a stuck task to pending state.

    Clears failure count and sets status back to pending.
    Useful when a task repeatedly fails due to transient issues.

    Example:

        afk reset auth-login
    """
    from afk.progress import SessionProgress

    progress = SessionProgress.load()

    if task_id not in progress.tasks:
        console.print(f"[red]Task not found:[/red] {task_id}")
        return

    task = progress.tasks[task_id]
    old_status = task.status
    old_failures = task.failure_count

    task.status = "pending"
    task.failure_count = 0
    task.started_at = None
    task.completed_at = None
    progress.save()

    console.print(f"[green]Task reset:[/green] {task_id}")
    console.print(f"[dim]Was: {old_status} with {old_failures} failures[/dim]")


@main.command()
@click.option("--verbose", "-v", is_flag=True, help="Show detailed information")
@click.pass_context
def explain(ctx: click.Context, verbose: bool) -> None:
    """Explain current loop state for debugging.

    Shows what task would be selected next, why, and session statistics.
    """
    from afk.prd_store import UserStory, load_prd
    from afk.progress import SessionProgress
    from afk.sources import aggregate_tasks

    config: AfkConfig = ctx.obj["config"]

    if not AFK_DIR.exists():
        console.print("[red]afk not initialized.[/red] Run [cyan]afk init[/cyan] first.")
        return

    progress = SessionProgress.load()

    # Get stories - all sources return UserStory, or read from PRD directly
    stories: list[UserStory]
    if config.sources:
        stories = aggregate_tasks(config.sources)
    else:
        # Zero-config mode: read directly from PRD
        prd = load_prd()
        stories = list(prd.user_stories)

    # Completion is tracked via passes field (UserStory.passes)
    completed = [s for s in stories if s.passes]
    pending = [s for s in stories if not s.passes]

    # Session info
    console.print(Panel.fit("[bold]Session State[/bold]", title="afk explain"))
    console.print()
    console.print(f"  [cyan]Iterations:[/cyan] {progress.iterations}")
    started = progress.started_at[:19] if progress.started_at else "N/A"
    console.print(f"  [cyan]Started:[/cyan] {started}")
    console.print()

    # Task breakdown
    failed_tasks = [t for t in progress.tasks.values() if t.status == "failed"]
    skipped_tasks = [t for t in progress.tasks.values() if t.status == "skipped"]

    console.print("  [cyan]Tasks:[/cyan]")
    console.print(f"    Total: {len(stories)}")
    console.print(f"    Completed: [green]{len(completed)}[/green]")
    console.print(f"    Pending: {len(pending)}")
    if failed_tasks:
        console.print(f"    Failed: [red]{len(failed_tasks)}[/red]")
    if skipped_tasks:
        console.print(f"    Skipped: [yellow]{len(skipped_tasks)}[/yellow]")
    console.print()

    # Next task
    if pending:
        # Sort by priority (1=highest, 5=lowest)
        pending.sort(key=lambda s: s.priority)
        next_story = pending[0]
        console.print("  [cyan]Next task:[/cyan]")
        console.print(f"    ID: [bold]{next_story.id}[/bold]")
        priority_str = {1: "HIGH", 2: "MEDIUM", 3: "MEDIUM", 4: "LOW", 5: "LOW"}.get(
            next_story.priority, "MEDIUM"
        )
        console.print(f"    Priority: {priority_str}")
        console.print(f"    Title: {next_story.title[:80]}...")
    else:
        console.print("  [green]All tasks complete![/green]")

    # Task learnings
    all_learnings = progress.get_all_learnings()
    if all_learnings:
        console.print()
        total_learnings = sum(len(items) for items in all_learnings.values())
        console.print(
            f"  [cyan]Learnings:[/cyan] {total_learnings} across {len(all_learnings)} tasks"
        )
        if verbose:
            for task_id, task_learnings in all_learnings.items():
                console.print(f"    [bold]{task_id}:[/bold]")
                for learning in task_learnings:
                    console.print(f"      - {learning}")

    # Failed tasks detail
    if verbose and failed_tasks:
        console.print()
        console.print("  [cyan]Failed tasks:[/cyan]")
        for task in failed_tasks:
            console.print(f"    - {task.id}: {task.failure_count} failures")
            if task.message:
                console.print(f"      [dim]{task.message}[/dim]")


@main.command()
@click.argument("iterations", type=int, default=10)
@click.option("--branch", "-b", help="Create/checkout feature branch")
@click.option("--yes", "-y", is_flag=True, help="Skip confirmation prompts")
@click.pass_context
def start(ctx: click.Context, iterations: int, branch: str | None, yes: bool) -> None:
    """Quick start: init if needed, then run the loop.

    Convenience command that combines init and run with sensible defaults.

    Examples:

        afk start                    # Init if needed, run 10 iterations

        afk start 20 -b my-feature   # Create branch, run 20 iterations

        afk start -y                 # Skip prompts, accept defaults
    """
    from afk.bootstrap import analyse_project, generate_config
    from afk.runner import LoopController

    config: AfkConfig = ctx.obj["config"]

    # Init if needed
    if not AFK_DIR.exists():
        console.print("[cyan]Initializing afk...[/cyan]")
        result = analyse_project()
        config = generate_config(result)
        AFK_DIR.mkdir(parents=True, exist_ok=True)
        config.save()
        console.print(f"[green]✓[/green] Configuration saved to {CONFIG_FILE}")

    # Check for sources
    if not config.sources:
        console.print()
        console.print("[yellow]No task sources configured.[/yellow]")
        console.print("Add sources with:")
        console.print("  [cyan]afk source add beads[/cyan]      # Use beads issue tracker")
        console.print("  [cyan]afk source add json prd.json[/cyan]  # Use JSON PRD file")
        console.print("  [cyan]afk source add markdown TODO.md[/cyan]  # Use markdown checklist")
        return

    # Confirm and run
    if not yes:
        console.print()
        console.print(f"Ready to run [cyan]{iterations}[/cyan] iterations")
        if branch:
            console.print(f"Branch: [cyan]{config.git.branch_prefix}{branch}[/cyan]")
        console.print()
        if not click.confirm("Start the loop?", default=True):
            console.print("[dim]Cancelled.[/dim]")
            return

    LoopController(config).run(
        max_iterations=iterations,
        branch=branch,
    )


@main.command()
@click.argument("iterations", type=int, default=5)
@click.option("--until-complete", "-u", is_flag=True, help="Run until all tasks complete")
@click.option("--timeout", "-t", type=int, help="Override timeout in minutes")
@click.option("--branch", "-b", help="Create/checkout feature branch")
@click.option("--continue", "-c", "resume_session", is_flag=True, help="Continue from last session")
@click.option(
    "--feedback",
    type=click.Choice(["full", "minimal", "off"]),
    default=None,
    help="Feedback display mode (default: from config or 'full')",
)
@click.pass_context
def run(
    ctx: click.Context,
    iterations: int,
    until_complete: bool,
    timeout: int | None,
    branch: str | None,
    resume_session: bool,
    feedback: str | None,
) -> None:
    """Run multiple iterations using configured AI CLI.

    Implements the Ralph Wiggum pattern: each iteration spawns a fresh
    AI CLI instance with clean context. Memory persists via git history,
    progress.json, and task sources.

    Examples:

        afk run 10                    # Run up to 10 iterations

        afk run --until-complete      # Run until all tasks done

        afk run 5 --branch my-feature # Create branch first

        afk run --timeout 60          # 60 minute timeout

        afk run --continue            # Resume from last session

        afk run --feedback minimal    # Use minimal feedback bar
    """
    from afk.runner import LoopController

    config: AfkConfig = ctx.obj["config"]

    if not AFK_DIR.exists():
        console.print("[red]afk not initialized.[/red] Run [cyan]afk init[/cyan] first.")
        return

    if not config.sources:
        console.print("[red]No sources configured.[/red] Run [cyan]afk source add[/cyan] first.")
        return

    # Determine feedback mode: CLI flag > config > default
    effective_feedback = feedback or config.feedback.mode

    # Run the autonomous loop
    LoopController(config).run(
        max_iterations=iterations,
        branch=branch,
        until_complete=until_complete,
        timeout_override=timeout,
        resume=resume_session,
        feedback_mode=effective_feedback,
    )


@main.command()
@click.argument("iterations", type=int, default=10)
@click.option("--until-complete", "-u", is_flag=True, help="Run until all tasks complete")
@click.option("--timeout", "-t", type=int, help="Override timeout in minutes")
@click.pass_context
def resume(
    ctx: click.Context,
    iterations: int,
    until_complete: bool,
    timeout: int | None,
) -> None:
    """Resume from last session without archiving.

    Continues the loop from where it left off, preserving existing
    progress and iteration count.

    Examples:

        afk resume                    # Continue with 10 more iterations

        afk resume 20                 # Continue with 20 more iterations

        afk resume --until-complete   # Continue until all tasks done
    """
    from afk.config import PROGRESS_FILE
    from afk.runner import LoopController

    config: AfkConfig = ctx.obj["config"]

    if not AFK_DIR.exists():
        console.print("[red]afk not initialized.[/red] Run [cyan]afk init[/cyan] first.")
        return

    if not config.sources:
        console.print("[red]No sources configured.[/red] Run [cyan]afk source add[/cyan] first.")
        return

    if not PROGRESS_FILE.exists():
        console.print("[yellow]No session to resume.[/yellow] Starting fresh.")

    # Run the autonomous loop with resume flag
    LoopController(config).run(
        max_iterations=iterations,
        until_complete=until_complete,
        timeout_override=timeout,
        resume=True,
    )


@main.group()
def archive() -> None:
    """Manage session archives."""
    pass


@archive.command("create")
@click.option("--reason", "-r", default="manual", help="Reason for archiving")
@click.pass_context
def archive_create(ctx: click.Context, reason: str) -> None:
    """Archive current session.

    Saves progress.json and prompt.md to a timestamped directory
    for later reference or recovery.
    """
    from afk.git_ops import archive_session

    config: AfkConfig = ctx.obj["config"]

    archive_path = archive_session(config, reason=reason)

    if archive_path:
        console.print(f"[green]Session archived to:[/green] {archive_path}")
    else:
        console.print("[yellow]Archiving disabled in config.[/yellow]")


@archive.command("list")
@click.pass_context
def archive_list(ctx: click.Context) -> None:
    """List archived sessions."""

    config: AfkConfig = ctx.obj["config"]
    archive_dir = Path(config.archive.directory)

    if not archive_dir.exists():
        console.print("[dim]No archives found.[/dim]")
        return

    archives = sorted(archive_dir.iterdir(), reverse=True)

    if not archives:
        console.print("[dim]No archives found.[/dim]")
        return

    table = Table(title="Session Archives", show_header=True)
    table.add_column("Archive", style="cyan")
    table.add_column("Date", style="white")
    table.add_column("Reason", style="dim")

    for archive_path in archives[:20]:  # Show last 20
        if archive_path.is_dir():
            metadata_file = archive_path / "metadata.json"
            if metadata_file.exists():
                import json

                with open(metadata_file) as f:
                    metadata = json.load(f)
                table.add_row(
                    archive_path.name,
                    metadata.get("archived_at", "?")[:19],
                    metadata.get("reason", "?"),
                )
            else:
                table.add_row(archive_path.name, "?", "?")

    console.print(table)


@archive.command("clear")
@click.option("--yes", "-y", is_flag=True, help="Skip confirmation")
@click.pass_context
def archive_clear(ctx: click.Context, yes: bool) -> None:
    """Clear current session progress.

    Removes progress.json to start fresh. Optionally archives first.
    """
    from afk.config import PROGRESS_FILE
    from afk.git_ops import archive_session, clear_session

    config: AfkConfig = ctx.obj["config"]

    if not PROGRESS_FILE.exists():
        console.print("[dim]No active session.[/dim]")
        return

    if not yes:
        if not click.confirm("Archive and clear current session?", default=True):
            console.print("[dim]Cancelled.[/dim]")
            return

    # Archive first
    archive_path = archive_session(config, reason="cleared")
    if archive_path:
        console.print(f"[dim]Archived to: {archive_path}[/dim]")

    clear_session()
    console.print("[green]Session cleared.[/green]")


@main.command()
@click.option("--beta", is_flag=True, help="Update to beta channel (pre-releases)")
@click.option("--check", is_flag=True, help="Check for updates without installing")
def update(beta: bool, check: bool) -> None:
    """Update afk to the latest version.

    Downloads and installs the latest release from GitHub. By default,
    only stable releases are considered. Use --beta to include pre-releases.

    Examples:

        afk update            # Update to latest stable

        afk update --beta     # Update to latest beta

        afk update --check    # Check for updates without installing
    """
    import platform
    import sys
    import tempfile
    import urllib.request

    from afk import __version__

    REPO = "m0nkmaster/afk"

    def get_platform_binary() -> str:
        """Get the binary name for the current platform."""
        system = platform.system().lower()
        machine = platform.machine().lower()

        if system == "darwin":
            os_name = "darwin"
        elif system == "linux":
            os_name = "linux"
        elif system == "windows":
            os_name = "windows"
        else:
            console.print(f"[red]Unsupported OS: {system}[/red]")
            sys.exit(1)

        if machine in ("x86_64", "amd64"):
            arch = "x86_64"
        elif machine in ("arm64", "aarch64"):
            arch = "arm64"
        else:
            console.print(f"[red]Unsupported architecture: {machine}[/red]")
            sys.exit(1)

        binary_name = f"afk-{os_name}-{arch}"
        if system == "windows":
            binary_name += ".exe"

        return binary_name

    def get_latest_version() -> tuple[str, str]:
        """Get latest version and download URL."""
        import json

        api_url = f"https://api.github.com/repos/{REPO}/releases"

        try:
            if beta:
                # Get all releases, including pre-releases
                with urllib.request.urlopen(api_url, timeout=10) as response:
                    releases = json.loads(response.read().decode())
                    if not releases:
                        console.print("[red]No releases found[/red]")
                        sys.exit(1)
                    latest = releases[0]
            else:
                # Get latest stable release
                with urllib.request.urlopen(f"{api_url}/latest", timeout=10) as response:
                    latest = json.loads(response.read().decode())

            return latest["tag_name"], latest["html_url"]

        except urllib.error.URLError as e:
            console.print(f"[red]Failed to check for updates: {e}[/red]")
            sys.exit(1)

    def download_and_install(version: str) -> None:
        """Download and install the new version."""
        binary_name = get_platform_binary()
        download_url = f"https://github.com/{REPO}/releases/download/{version}/{binary_name}"

        console.print(f"[cyan]Downloading {binary_name}...[/cyan]")

        try:
            # Download to temp file
            with tempfile.NamedTemporaryFile(delete=False, suffix=".tmp") as tmp:
                with urllib.request.urlopen(download_url, timeout=60) as response:
                    tmp.write(response.read())
                tmp_path = Path(tmp.name)

            # Get current executable path
            current_exe = Path(sys.executable)
            if current_exe.name == "python" or current_exe.name.startswith("python"):
                # Running from pip install, not standalone binary
                console.print("[yellow]Running from Python installation.[/yellow]")
                console.print("For standalone binary updates, reinstall with:")
                console.print()
                if platform.system().lower() == "windows":
                    console.print(
                        "  [cyan]irm https://raw.githubusercontent.com/"
                        "m0nkmaster/afk/main/scripts/install.ps1 | iex[/cyan]"
                    )
                else:
                    console.print(
                        "  [cyan]curl -fsSL https://raw.githubusercontent.com/"
                        "m0nkmaster/afk/main/scripts/install.sh | bash[/cyan]"
                    )
                tmp_path.unlink()
                return

            # Replace current binary
            console.print("[cyan]Installing...[/cyan]")

            if platform.system().lower() == "windows":
                # Windows: rename current, move new, delete old
                backup_path = current_exe.with_suffix(".old")
                current_exe.rename(backup_path)
                tmp_path.rename(current_exe)
                backup_path.unlink()
            else:
                # Unix: just replace
                import os
                import stat

                tmp_path.chmod(tmp_path.stat().st_mode | stat.S_IXUSR | stat.S_IXGRP | stat.S_IXOTH)
                os.replace(str(tmp_path), str(current_exe))

            console.print(f"[green]✓ Updated to {version}[/green]")

        except urllib.error.URLError as e:
            console.print(f"[red]Download failed: {e}[/red]")
            sys.exit(1)
        except OSError as e:
            console.print(f"[red]Installation failed: {e}[/red]")
            console.print("[dim]You may need to run with elevated privileges[/dim]")
            sys.exit(1)

    # Check for updates
    console.print(f"[dim]Current version: {__version__}[/dim]")

    latest_version, release_url = get_latest_version()
    latest_version_clean = latest_version.lstrip("v")

    if latest_version_clean == __version__:
        console.print("[green]✓ Already up to date[/green]")
        return

    console.print(f"[cyan]New version available: {latest_version}[/cyan]")

    if check:
        console.print(f"[dim]Release: {release_url}[/dim]")
        return

    download_and_install(latest_version)


@main.command()
@click.argument("shell", type=click.Choice(["bash", "zsh", "fish"]))
def completions(shell: str) -> None:
    """Generate shell completions.

    Outputs completion script to stdout. Redirect to appropriate file
    for your shell, or use the install script which does this automatically.

    Examples:

        afk completions bash > ~/.local/share/bash-completion/completions/afk

        afk completions zsh > ~/.local/share/zsh/site-functions/_afk

        afk completions fish > ~/.config/fish/completions/afk.fish
    """
    import sys

    from click.shell_completion import get_completion_class

    comp_cls = get_completion_class(shell)
    if comp_cls is None:
        console.print(f"[red]Unsupported shell: {shell}[/red]")
        sys.exit(1)

    # Create completion instance
    comp = comp_cls(main, {}, "afk", "_AFK_COMPLETE")

    # Output the completion script
    click.echo(comp.source())


if __name__ == "__main__":
    main()

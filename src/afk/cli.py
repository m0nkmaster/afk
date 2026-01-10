"""CLI interface for afk."""

from __future__ import annotations

from pathlib import Path

import click
from rich.console import Console
from rich.panel import Panel
from rich.table import Table

from afk import __version__
from afk.config import AFK_DIR, CONFIG_FILE, LEARNINGS_FILE, AfkConfig

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
@click.pass_context
def go_command(
    ctx: click.Context,
    iterations_or_source: str | None,
    iterations_if_source: int | None,
    dry_run: bool,
    until_complete: bool,
) -> None:
    """Quick start with zero config.

    \b
    Examples:
      afk go                 # Auto-detect, run 10 iterations
      afk go 20              # Auto-detect, run 20 iterations
      afk go -u              # Run until all tasks complete
      afk go TODO.md 5       # Use TODO.md as source, run 5 iterations
    """
    _run_zero_config(ctx, iterations_or_source, iterations_if_source, dry_run, until_complete)


def _run_zero_config(
    ctx: click.Context,
    iterations_or_source: str | None,
    iterations_if_source: int | None,
    dry_run: bool,
    until_complete: bool = False,
) -> None:
    """Handle zero-config invocation (afk go, afk go 10, afk go TODO.md 5)."""
    from afk.bootstrap import (
        detect_prompt_file,
        ensure_ai_cli_configured,
        infer_config,
        infer_sources,
    )
    from afk.runner import run_loop

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
        if not existing_prd.userStories:
            # No PRD exists, try to infer sources
            inferred = infer_sources()
            if inferred:
                config.sources = inferred

    # Check if we have work to do (either sources or existing PRD)
    if not config.sources:
        from afk.prd_store import load_prd

        existing_prd = load_prd()
        if not existing_prd.userStories:
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

    # Run the loop
    run_loop(config=config, max_iterations=iterations, until_complete=until_complete)


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

    pending = sum(1 for s in prd.userStories if not s.passes)
    complete = sum(1 for s in prd.userStories if s.passes)

    console.print()
    console.print("[green]PRD synced:[/green] .afk/prd.json")
    console.print(f"  Stories: {len(prd.userStories)} total")
    console.print(f"  Pending: {pending}")
    console.print(f"  Complete: {complete}")

    if prd.branchName:
        console.print(f"  Branch: {prd.branchName}")


@prd.command("show")
@click.option("--pending", "-p", is_flag=True, help="Show only pending stories")
@click.pass_context
def prd_show(ctx: click.Context, pending: bool) -> None:
    """Show the current PRD state.

    Displays user stories from .afk/prd.json with their completion status.
    """
    from afk.prd_store import load_prd

    prd = load_prd()

    if not prd.userStories:
        console.print("[dim]No stories in PRD.[/dim] Run [cyan]afk prd sync[/cyan] first.")
        return

    table = Table(title=f"PRD: {prd.project or 'Untitled'}", show_header=True)
    table.add_column("ID", style="cyan")
    table.add_column("Pri", style="dim", width=3)
    table.add_column("Title", style="white")
    table.add_column("AC", style="dim", width=3)
    table.add_column("Status", style="white")

    for story in prd.userStories:
        if pending and story.passes:
            continue

        status = "[green]✓ Pass[/green]" if story.passes else "[dim]Pending[/dim]"
        ac_count = str(len(story.acceptanceCriteria))

        table.add_row(
            story.id,
            str(story.priority),
            story.title[:50] + ("..." if len(story.title) > 50 else ""),
            ac_count,
            status,
        )

    console.print(table)

    if prd.branchName:
        console.print(f"\n[dim]Branch: {prd.branchName}[/dim]")
    if prd.lastSynced:
        console.print(f"[dim]Last synced: {prd.lastSynced[:19]}[/dim]")


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

    pending = sum(1 for s in prd.userStories if not s.passes)
    complete = sum(1 for s in prd.userStories if s.passes)

    console.print()
    console.print("[green]PRD synced:[/green] .afk/prd.json")
    console.print(f"  Stories: {len(prd.userStories)} total")
    console.print(f"  Pending: {pending}")
    console.print(f"  Complete: {complete}")

    if prd.branchName:
        console.print(f"  Branch: {prd.branchName}")


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
@click.argument("task_id")
@click.option("--message", "-m", help="Failure reason")
@click.pass_context
def fail(ctx: click.Context, task_id: str, message: str | None) -> None:
    """Mark a task as failed."""
    from afk.progress import mark_failed

    failure_count = mark_failed(task_id, message=message)
    console.print(f"[red]Task failed:[/red] {task_id} (attempt {failure_count})")


@main.command()
@click.argument("content")
@click.option("--task", "-t", help="Associate learning with a task ID")
def learn(content: str, task: str | None) -> None:
    """Record a learning for future iterations.

    Learnings persist across sessions and are included in every prompt.
    Use this to record patterns, gotchas, and discoveries.

    Examples:

        afk learn "This codebase uses factory pattern for services"

        afk learn "Must run migrations before tests" --task db-setup
    """
    from afk.learnings import append_learning

    append_learning(content, task_id=task)
    console.print("[green]Learning recorded.[/green]")

    if LEARNINGS_FILE.exists():
        line_count = len(LEARNINGS_FILE.read_text().strip().split("\n"))
        console.print(f"[dim]Total learnings: {line_count} lines[/dim]")


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
    from afk.learnings import load_learnings
    from afk.progress import SessionProgress
    from afk.sources import aggregate_tasks

    config: AfkConfig = ctx.obj["config"]

    if not AFK_DIR.exists():
        console.print("[red]afk not initialized.[/red] Run [cyan]afk init[/cyan] first.")
        return

    progress = SessionProgress.load()
    tasks = aggregate_tasks(config.sources) if config.sources else []

    # Session info
    console.print(Panel.fit("[bold]Session State[/bold]", title="afk explain"))
    console.print()
    console.print(f"  [cyan]Iterations:[/cyan] {progress.iterations}")
    started = progress.started_at[:19] if progress.started_at else "N/A"
    console.print(f"  [cyan]Started:[/cyan] {started}")
    console.print()

    # Task breakdown
    completed_ids = {t.id for t in progress.get_completed_tasks()}
    failed_tasks = [t for t in progress.tasks.values() if t.status == "failed"]
    skipped_tasks = [t for t in progress.tasks.values() if t.status == "skipped"]
    pending_tasks = [t for t in tasks if t.id not in completed_ids]

    console.print("  [cyan]Tasks:[/cyan]")
    console.print(f"    Total: {len(tasks)}")
    console.print(f"    Completed: [green]{len(completed_ids)}[/green]")
    console.print(f"    Pending: {len(pending_tasks)}")
    if failed_tasks:
        console.print(f"    Failed: [red]{len(failed_tasks)}[/red]")
    if skipped_tasks:
        console.print(f"    Skipped: [yellow]{len(skipped_tasks)}[/yellow]")
    console.print()

    # Next task
    if pending_tasks:
        priority_order = {"high": 0, "medium": 1, "low": 2}
        pending_tasks.sort(key=lambda t: priority_order.get(t.priority, 1))
        next_task = pending_tasks[0]
        console.print("  [cyan]Next task:[/cyan]")
        console.print(f"    ID: [bold]{next_task.id}[/bold]")
        console.print(f"    Priority: {next_task.priority.upper()}")
        console.print(f"    Description: {next_task.description[:80]}...")
    else:
        console.print("  [green]All tasks complete![/green]")

    # Learnings
    learnings = load_learnings()
    if learnings:
        console.print()
        console.print(f"  [cyan]Learnings:[/cyan] {len(learnings)} chars")
        if verbose:
            console.print()
            recent = learnings[-500:] if len(learnings) > 500 else learnings
            console.print(Panel(recent, title="Recent Learnings"))

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
    from afk.runner import run_loop

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

    run_loop(
        config=config,
        max_iterations=iterations,
        branch=branch,
    )


@main.command()
@click.argument("iterations", type=int, default=5)
@click.option("--until-complete", "-u", is_flag=True, help="Run until all tasks complete")
@click.option("--timeout", "-t", type=int, help="Override timeout in minutes")
@click.option("--branch", "-b", help="Create/checkout feature branch")
@click.option("--continue", "-c", "resume_session", is_flag=True, help="Continue from last session")
@click.pass_context
def run(
    ctx: click.Context,
    iterations: int,
    until_complete: bool,
    timeout: int | None,
    branch: str | None,
    resume_session: bool,
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
    """
    from afk.runner import run_loop

    config: AfkConfig = ctx.obj["config"]

    if not AFK_DIR.exists():
        console.print("[red]afk not initialized.[/red] Run [cyan]afk init[/cyan] first.")
        return

    if not config.sources:
        console.print("[red]No sources configured.[/red] Run [cyan]afk source add[/cyan] first.")
        return

    # Run the autonomous loop
    run_loop(
        config=config,
        max_iterations=iterations,
        branch=branch,
        until_complete=until_complete,
        timeout_override=timeout,
        resume=resume_session,
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
    from afk.runner import run_loop

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
    run_loop(
        config=config,
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


if __name__ == "__main__":
    main()

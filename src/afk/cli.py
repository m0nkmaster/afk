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


@click.group()
@click.version_option(version=__version__, prog_name="afk")
@click.pass_context
def main(ctx: click.Context) -> None:
    """afk - Autonomous AI coding loops.

    Run AI coding tasks in a loop, Ralph Wiggum style.
    """
    ctx.ensure_object(dict)
    ctx.obj["config"] = AfkConfig.load()


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
@click.argument("iterations", type=int, default=5)
@click.option("--until-complete", "-u", is_flag=True, help="Run until all tasks complete")
@click.option("--timeout", "-t", type=int, help="Override timeout in minutes")
@click.option("--branch", "-b", help="Create/checkout feature branch")
@click.pass_context
def run(
    ctx: click.Context,
    iterations: int,
    until_complete: bool,
    timeout: int | None,
    branch: str | None,
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

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
@click.option("--force", "-f", is_flag=True, help="Overwrite existing config")
def init(force: bool) -> None:
    """Initialize afk in the current directory."""
    if AFK_DIR.exists() and not force:
        console.print("[yellow]afk already initialized.[/yellow] Use --force to reinitialize.")
        return

    AFK_DIR.mkdir(parents=True, exist_ok=True)

    config = AfkConfig()
    config.save()

    console.print(Panel.fit(
        "[green]afk initialized![/green]\n\n"
        f"Config: [cyan]{CONFIG_FILE}[/cyan]\n\n"
        "Next steps:\n"
        "  1. Add task sources: [cyan]afk source add beads[/cyan]\n"
        "  2. Check status: [cyan]afk status[/cyan]\n"
        "  3. Get next prompt: [cyan]afk next[/cyan]",
        title="afk",
    ))


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
    from afk.prompt import generate_prompt
    from afk.output import output_prompt

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
@click.argument("iterations", type=int, default=5)
@click.option("--until-complete", is_flag=True, help="Run until all tasks complete")
@click.option("--timeout", type=int, help="Override timeout in minutes")
@click.pass_context
def run(
    ctx: click.Context,
    iterations: int,
    until_complete: bool,
    timeout: int | None,
) -> None:
    """Run multiple iterations using configured AI CLI."""
    config: AfkConfig = ctx.obj["config"]

    console.print(
        f"[yellow]Running {iterations} iterations with {config.ai_cli.command}...[/yellow]"
    )
    console.print("[dim]This feature is coming soon.[/dim]")


if __name__ == "__main__":
    main()

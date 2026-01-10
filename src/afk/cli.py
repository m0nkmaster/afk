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

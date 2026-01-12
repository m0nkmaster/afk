"""Output handlers for afk prompts."""

from __future__ import annotations

from pathlib import Path
from typing import Literal

from rich.console import Console

from afk.config import AfkConfig

console = Console()


def output_prompt(
    prompt: str,
    mode: Literal["clipboard", "file", "stdout"],
    config: AfkConfig,
) -> None:
    """Output the generated prompt."""
    if mode == "clipboard":
        _copy_to_clipboard(prompt)
    elif mode == "file":
        _write_to_file(prompt, config.output.file_path)
    else:
        _print_to_stdout(prompt)


def _copy_to_clipboard(prompt: str) -> None:
    """Copy prompt to system clipboard."""
    try:
        import pyperclip

        pyperclip.copy(prompt)
        console.print("[green]Prompt copied to clipboard![/green]")
        console.print(f"[dim]({len(prompt)} characters)[/dim]")
    except Exception as e:
        console.print(f"[red]Failed to copy to clipboard:[/red] {e}")
        console.print("[dim]Falling back to stdout...[/dim]")
        _print_to_stdout(prompt)


def _write_to_file(prompt: str, file_path: str) -> None:
    """Write prompt to file."""
    path = Path(file_path)
    path.parent.mkdir(parents=True, exist_ok=True)

    with open(path, "w") as f:
        f.write(prompt)

    console.print(f"[green]Prompt written to:[/green] {path}")
    console.print(f"[dim]Include with: @{path}[/dim]")


def _print_to_stdout(prompt: str) -> None:
    """Print prompt to stdout."""
    console.print(prompt)

"""Project bootstrap and auto-configuration for afk."""

from __future__ import annotations

import shutil
import subprocess
from dataclasses import dataclass, field
from pathlib import Path
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from rich.console import Console

from afk.config import (
    AFK_DIR,
    CONFIG_FILE,
    AfkConfig,
    AiCliConfig,
    FeedbackLoopsConfig,
    OutputConfig,
    PromptConfig,
    SourceConfig,
)


@dataclass
class AiCliInfo:
    """Information about an AI CLI tool."""

    command: str
    name: str
    args: list[str]
    description: str
    install_url: str


@dataclass
class ProjectStack:
    """Detected project technology stack."""

    name: str
    config_file: str
    feedback_loops: FeedbackLoopsConfig = field(default_factory=FeedbackLoopsConfig)


@dataclass
class BootstrapResult:
    """Result of project analysis."""

    stack: ProjectStack | None = None
    sources: list[SourceConfig] = field(default_factory=list)
    context_files: list[str] = field(default_factory=list)
    available_tools: dict[str, bool] = field(default_factory=dict)
    warnings: list[str] = field(default_factory=list)


# Stack definitions with their feedback loops
STACKS: dict[str, tuple[str, FeedbackLoopsConfig]] = {
    "pyproject.toml": (
        "Python",
        FeedbackLoopsConfig(
            lint="ruff check .",
            types="mypy .",
            test="pytest",
        ),
    ),
    "setup.py": (
        "Python (legacy)",
        FeedbackLoopsConfig(
            lint="ruff check .",
            types="mypy .",
            test="pytest",
        ),
    ),
    "package.json": (
        "Node.js",
        FeedbackLoopsConfig(
            lint="npm run lint",
            test="npm test",
            build="npm run build",
        ),
    ),
    "Cargo.toml": (
        "Rust",
        FeedbackLoopsConfig(
            lint="cargo clippy",
            test="cargo test",
            build="cargo build",
        ),
    ),
    "go.mod": (
        "Go",
        FeedbackLoopsConfig(
            lint="go vet ./...",
            test="go test ./...",
            build="go build ./...",
        ),
    ),
    "pom.xml": (
        "Java (Maven)",
        FeedbackLoopsConfig(
            test="mvn test",
            build="mvn compile",
        ),
    ),
    "build.gradle": (
        "Java (Gradle)",
        FeedbackLoopsConfig(
            test="gradle test",
            build="gradle build",
        ),
    ),
}

# Context files to look for (in priority order)
CONTEXT_FILES = [
    "AGENTS.md",
    "CLAUDE.md",
    "CURSOR.md",
    "CONTRIBUTING.md",
    "README.md",
]

# Task source files to detect
TASK_FILES = {
    "prd.json": "json",
    "tasks.json": "json",
    "TODO.md": "markdown",
    "TASKS.md": "markdown",
    "tasks.md": "markdown",
}

# Prompt files for single-prompt mode (ralf.sh style)
PROMPT_FILES = [
    "prompt.md",
    "PROMPT.md",
    "prompt.txt",
]

# Known AI CLI tools with metadata
# Args are configured for autonomous (non-interactive) operation.
# The prompt is passed as the final command-line argument after these args.
AI_CLIS: list[AiCliInfo] = [
    AiCliInfo(
        command="claude",
        name="Claude Code",
        args=["--dangerously-skip-permissions"],
        description="Anthropic's Claude CLI for autonomous terminal-based AI coding",
        install_url="https://docs.anthropic.com/en/docs/claude-code",
    ),
    AiCliInfo(
        command="agent",
        name="Cursor Agent",
        args=["-p", "--force"],
        description="Cursor's CLI agent for autonomous terminal-based AI coding",
        install_url="https://docs.cursor.com/cli",
    ),
    AiCliInfo(
        command="codex",
        name="Codex",
        args=["--approval-mode", "full-auto"],
        description="OpenAI's Codex CLI for terminal-based AI coding",
        install_url="https://github.com/openai/codex",
    ),
    AiCliInfo(
        command="aider",
        name="Aider",
        args=["--yes", "--message"],
        description="AI pair programming in your terminal",
        install_url="https://aider.chat",
    ),
    AiCliInfo(
        command="amp",
        name="Amp",
        args=["--dangerously-allow-all"],
        description="Sourcegraph's agentic coding tool",
        install_url="https://sourcegraph.com/amp",
    ),
    AiCliInfo(
        command="kiro",
        name="Kiro",
        args=["--auto"],
        description="Amazon's AI-powered development CLI for terminal-based coding",
        install_url="https://kiro.dev",
    ),
]


def _command_exists(cmd: str) -> bool:
    """Check if a command is available on PATH."""
    return shutil.which(cmd) is not None


def _detect_stack(root: Path) -> ProjectStack | None:
    """Detect the project's technology stack."""
    for config_file, (name, loops) in STACKS.items():
        if (root / config_file).exists():
            return ProjectStack(name=name, config_file=config_file, feedback_loops=loops)
    return None


def _detect_sources(root: Path, available_tools: dict[str, bool]) -> list[SourceConfig]:
    """Detect available task sources."""
    sources: list[SourceConfig] = []

    # Check for beads
    if (root / ".beads").is_dir() and available_tools.get("bd"):
        sources.append(SourceConfig(type="beads"))

    # Check for .afk/prd.json first (created by afk prd parse)
    afk_prd = root / ".afk" / "prd.json"
    if afk_prd.exists():
        sources.append(SourceConfig(type="json", path=".afk/prd.json"))

    # Check for task files in project root
    for filename, source_type in TASK_FILES.items():
        if (root / filename).exists():
            sources.append(SourceConfig(type=source_type, path=filename))  # type: ignore[arg-type]

    # Check for glob patterns (e.g., *.prd.json)
    for prd_file in root.glob("*.prd.json"):
        sources.append(SourceConfig(type="json", path=str(prd_file.name)))

    return sources


def detect_prompt_file(root: Path | None = None) -> Path | None:
    """Detect a single prompt file for ralf.sh-style mode.

    Args:
        root: Project root directory (defaults to cwd)

    Returns:
        Path to prompt file if found, None otherwise
    """
    if root is None:
        root = Path.cwd()

    for filename in PROMPT_FILES:
        path = root / filename
        if path.exists():
            return path

    return None


def infer_sources(root: Path | None = None) -> list[SourceConfig]:
    """Infer task sources from project structure without config.

    This is the zero-config entry point - detects sources automatically
    without requiring afk init or a config file.

    Args:
        root: Project root directory (defaults to cwd)

    Returns:
        List of detected SourceConfig objects
    """
    if root is None:
        root = Path.cwd()

    available_tools = _detect_tools()
    return _detect_sources(root, available_tools)


def _detect_context_files(root: Path) -> list[str]:
    """Detect available context files."""
    found = []
    for filename in CONTEXT_FILES:
        if (root / filename).exists():
            found.append(filename)
    return found


def _detect_tools() -> dict[str, bool]:
    """Detect available CLI tools."""
    tools = {
        "bd": _command_exists("bd"),
        "gh": _command_exists("gh"),
        "agent": _command_exists("agent"),
        "claude": _command_exists("claude"),
        "codex": _command_exists("codex"),
        "aider": _command_exists("aider"),
        "amp": _command_exists("amp"),
        "kiro": _command_exists("kiro"),
    }
    return tools


def _detect_ai_cli(tools: dict[str, bool]) -> AiCliConfig:
    """Detect the best available AI CLI tool.

    Priority order: claude > agent > codex > kiro > aider > amp
    """
    if tools.get("claude"):
        return AiCliConfig(command="claude", args=["--dangerously-skip-permissions", "-p"])
    if tools.get("agent"):
        return AiCliConfig(command="agent", args=["--force", "-p"])
    if tools.get("codex"):
        return AiCliConfig(command="codex", args=["--approval-mode", "full-auto", "-q"])
    if tools.get("kiro"):
        return AiCliConfig(command="kiro", args=["--auto"])
    if tools.get("aider"):
        return AiCliConfig(command="aider", args=["--yes"])
    if tools.get("amp"):
        return AiCliConfig(command="amp", args=["--dangerously-allow-all"])
    # Default - user will need to install one
    return AiCliConfig()


def detect_available_ai_clis() -> list[AiCliInfo]:
    """Detect which AI CLI tools are installed.

    Returns:
        List of AiCliInfo for each installed AI CLI tool.
    """
    available = []
    for cli in AI_CLIS:
        if _command_exists(cli.command):
            available.append(cli)
    return available


def prompt_ai_cli_selection(
    available: list[AiCliInfo],
    console: Console | None = None,
) -> AiCliConfig | None:
    """Interactively prompt user to select an AI CLI.

    Args:
        available: List of available AI CLI tools
        console: Optional Rich console for output

    Returns:
        Selected AiCliConfig, or None if user cancels
    """
    if console is None:
        from rich.console import Console

        console = Console()

    if not available:
        console.print()
        console.print("[red]No AI CLI tools found.[/red]")
        console.print()
        console.print("Install one of the following:")
        for cli in AI_CLIS:
            console.print(f"  • [cyan]{cli.name}[/cyan]: {cli.install_url}")
        console.print()
        return None

    console.print()
    console.print("[bold]Welcome to afk![/bold]")
    console.print()
    console.print("Detected AI tools:")
    for i, cli in enumerate(available, 1):
        console.print(f"  [cyan]{i}[/cyan]. {cli.name} [dim]({cli.description})[/dim]")
    console.print()

    # Prompt for selection
    import click

    default = 1
    choice = click.prompt(
        "Which AI CLI should afk use?",
        type=click.IntRange(1, len(available)),
        default=default,
        show_default=True,
    )

    selected = available[choice - 1]
    return AiCliConfig(command=selected.command, args=selected.args)


def ensure_ai_cli_configured(
    config: AfkConfig | None = None,
    console: Console | None = None,
) -> AiCliConfig:
    """Ensure AI CLI is configured, prompting user if needed.

    This is the main entry point for the first-run experience.
    If config exists with ai_cli set, returns it.
    Otherwise, detects available CLIs and prompts user to choose.

    Args:
        config: Existing config (or None to load from file)
        console: Optional Rich console for output

    Returns:
        AiCliConfig (either from config or user selection)

    Raises:
        SystemExit: If no AI CLI available and user cannot select one
    """
    import sys

    if console is None:
        from rich.console import Console

        console = Console()

    # Load config if not provided
    if config is None:
        config = AfkConfig.load()

    # Check if AI CLI is already configured (non-default)
    # We check if config file exists AND has explicit ai_cli
    if CONFIG_FILE.exists():
        # Config exists, use what's there
        return config.ai_cli

    # First run - need to prompt
    available = detect_available_ai_clis()
    selected = prompt_ai_cli_selection(available, console)

    if selected is None:
        # No AI CLIs available
        sys.exit(1)

    # Save the selection to config
    config.ai_cli = selected
    AFK_DIR.mkdir(parents=True, exist_ok=True)
    config.save()

    console.print()
    console.print(f"[green]✓[/green] Saved AI CLI choice: [cyan]{selected.command}[/cyan]")
    console.print(f"  Config: [dim]{CONFIG_FILE}[/dim]")
    console.print()

    return selected


def _is_github_repo(root: Path) -> bool:
    """Check if this is a GitHub repository."""
    try:
        result = subprocess.run(
            ["git", "remote", "get-url", "origin"],
            capture_output=True,
            text=True,
            cwd=root,
        )
        if result.returncode == 0:
            url = result.stdout.strip()
            return "github.com" in url
    except (subprocess.SubprocessError, FileNotFoundError):
        pass
    return False


def analyse_project(root: Path | None = None) -> BootstrapResult:
    """Analyse a project directory and return bootstrap recommendations."""
    if root is None:
        root = Path.cwd()

    result = BootstrapResult()

    # Detect available tools
    result.available_tools = _detect_tools()

    # Detect stack
    result.stack = _detect_stack(root)
    if not result.stack:
        result.warnings.append("Could not detect project type. Feedback loops not configured.")

    # Detect sources
    result.sources = _detect_sources(root, result.available_tools)

    # Add GitHub as an option if available
    if result.available_tools.get("gh") and _is_github_repo(root):
        # Don't auto-add, but note it's available
        pass

    if not result.sources:
        result.warnings.append(
            "No task sources detected. Add sources with 'afk source add' or create a TODO.md file."
        )

    # Detect context files
    result.context_files = _detect_context_files(root)
    if not result.context_files:
        result.warnings.append("No context files found. Consider creating AGENTS.md or README.md.")

    return result


def generate_config(result: BootstrapResult) -> AfkConfig:
    """Generate an AfkConfig from bootstrap analysis."""
    feedback_loops = result.stack.feedback_loops if result.stack else FeedbackLoopsConfig()

    ai_cli = _detect_ai_cli(result.available_tools)

    return AfkConfig(
        sources=result.sources,
        feedback_loops=feedback_loops,
        output=OutputConfig(default="clipboard"),
        ai_cli=ai_cli,
        prompt=PromptConfig(
            context_files=result.context_files,
        ),
    )


def infer_config(root: Path | None = None) -> AfkConfig:
    """Infer a complete config from project structure without writing to disk.

    This is the zero-config entry point - creates a usable config by
    detecting project type, sources, and available tools automatically.

    If a config file exists, loads and returns it instead.

    Args:
        root: Project root directory (defaults to cwd)

    Returns:
        AfkConfig ready for use
    """
    if root is None:
        root = Path.cwd()

    # If config exists, use it
    if CONFIG_FILE.exists():
        return AfkConfig.load()

    # Otherwise, infer everything
    result = analyse_project(root)
    return generate_config(result)

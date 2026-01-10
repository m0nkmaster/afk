"""Project bootstrap and auto-configuration for afk."""

from __future__ import annotations

import shutil
import subprocess
from dataclasses import dataclass, field
from pathlib import Path

from afk.config import (
    AfkConfig,
    AiCliConfig,
    FeedbackLoopsConfig,
    OutputConfig,
    PromptConfig,
    SourceConfig,
)


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

    # Check for task files
    for filename, source_type in TASK_FILES.items():
        if (root / filename).exists():
            sources.append(SourceConfig(type=source_type, path=filename))  # type: ignore[arg-type]

    # Check for glob patterns (e.g., *.prd.json)
    for prd_file in root.glob("*.prd.json"):
        sources.append(SourceConfig(type="json", path=str(prd_file.name)))

    return sources


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
        "claude": _command_exists("claude"),
        "cursor": _command_exists("cursor"),
        "aider": _command_exists("aider"),
    }
    return tools


def _detect_ai_cli(tools: dict[str, bool]) -> AiCliConfig:
    """Detect the best available AI CLI tool."""
    if tools.get("claude"):
        return AiCliConfig(command="claude", args=["-p"])
    if tools.get("aider"):
        return AiCliConfig(command="aider", args=["--message"])
    # Default
    return AiCliConfig(command="claude", args=["-p"])


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

"""Configuration models for afk."""

from __future__ import annotations

import json
from pathlib import Path
from typing import Literal

from pydantic import BaseModel, Field


class SourceConfig(BaseModel):
    """Configuration for a task source."""

    type: Literal["beads", "json", "markdown", "github"]
    path: str | None = None
    # GitHub-specific options
    repo: str | None = None
    labels: list[str] = Field(default_factory=list)


class FeedbackLoopsConfig(BaseModel):
    """Configuration for feedback loop commands."""

    types: str | None = None
    lint: str | None = None
    test: str | None = None
    build: str | None = None
    custom: dict[str, str] = Field(default_factory=dict)


class LimitsConfig(BaseModel):
    """Configuration for iteration limits."""

    max_iterations: int = 20
    max_task_failures: int = 20
    timeout_minutes: int = 30


class OutputConfig(BaseModel):
    """Configuration for output modes."""

    default: Literal["clipboard", "file", "stdout"] = "stdout"
    file_path: str = ".afk/prompt.md"


class AiCliConfig(BaseModel):
    """Configuration for AI CLI integration."""

    command: str = "claude"
    args: list[str] = Field(default_factory=lambda: ["--dangerously-skip-permissions", "-p"])


class PromptConfig(BaseModel):
    """Configuration for prompt generation."""

    template: str = "default"
    custom_path: str | None = None
    context_files: list[str] = Field(default_factory=list)
    instructions: list[str] = Field(default_factory=list)


class GitConfig(BaseModel):
    """Configuration for git integration."""

    auto_commit: bool = True
    auto_branch: bool = False
    branch_prefix: str = "afk/"
    commit_message_template: str = "afk: {task_id} - {message}"


class ArchiveConfig(BaseModel):
    """Configuration for session archiving."""

    enabled: bool = True
    directory: str = ".afk/archive"
    on_branch_change: bool = True


class AfkConfig(BaseModel):
    """Main configuration for afk."""

    sources: list[SourceConfig] = Field(default_factory=list)
    feedback_loops: FeedbackLoopsConfig = Field(default_factory=FeedbackLoopsConfig)
    limits: LimitsConfig = Field(default_factory=LimitsConfig)
    output: OutputConfig = Field(default_factory=OutputConfig)
    ai_cli: AiCliConfig = Field(default_factory=AiCliConfig)
    prompt: PromptConfig = Field(default_factory=PromptConfig)
    git: GitConfig = Field(default_factory=GitConfig)
    archive: ArchiveConfig = Field(default_factory=ArchiveConfig)

    @classmethod
    def load(cls, path: Path | None = None) -> AfkConfig:
        """Load configuration from file or return defaults."""
        if path is None:
            path = Path(".afk/config.json")

        if not path.exists():
            return cls()

        with open(path) as f:
            data = json.load(f)

        return cls.model_validate(data)

    def save(self, path: Path | None = None) -> None:
        """Save configuration to file."""
        if path is None:
            path = Path(".afk/config.json")

        path.parent.mkdir(parents=True, exist_ok=True)

        with open(path, "w") as f:
            json.dump(self.model_dump(exclude_none=True), f, indent=2)


# Default config directory
AFK_DIR = Path(".afk")
CONFIG_FILE = AFK_DIR / "config.json"
PROGRESS_FILE = AFK_DIR / "progress.json"
PROMPT_FILE = AFK_DIR / "prompt.md"
PRD_FILE = AFK_DIR / "prd.json"
ARCHIVE_DIR = AFK_DIR / "archive"

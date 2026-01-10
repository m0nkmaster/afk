"""Task source adapters for afk."""

from __future__ import annotations

from dataclasses import dataclass
from typing import Literal

from afk.config import SourceConfig


@dataclass
class Task:
    """A task from any source."""

    id: str
    description: str
    priority: Literal["high", "medium", "low"] = "medium"
    source: str = "unknown"
    metadata: dict | None = None


def aggregate_tasks(sources: list[SourceConfig]) -> list[Task]:
    """Aggregate tasks from all configured sources."""
    all_tasks: list[Task] = []

    for source in sources:
        tasks = _load_from_source(source)
        all_tasks.extend(tasks)

    return all_tasks


def _load_from_source(source: SourceConfig) -> list[Task]:
    """Load tasks from a single source."""
    if source.type == "beads":
        from afk.sources.beads import load_beads_tasks

        return load_beads_tasks()
    elif source.type == "json":
        from afk.sources.json_prd import load_json_tasks

        return load_json_tasks(source.path)
    elif source.type == "markdown":
        from afk.sources.markdown import load_markdown_tasks

        return load_markdown_tasks(source.path)
    elif source.type == "github":
        from afk.sources.github import load_github_tasks

        return load_github_tasks(source.repo, source.labels)
    else:
        return []

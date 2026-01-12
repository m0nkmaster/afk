"""Task source adapters for afk.

All sources return UserStory objects - the canonical task type.
"""

from __future__ import annotations

from afk.config import SourceConfig
from afk.prd_store import UserStory


def aggregate_tasks(sources: list[SourceConfig]) -> list[UserStory]:
    """Aggregate tasks from all configured sources."""
    all_tasks: list[UserStory] = []

    for source in sources:
        tasks = _load_from_source(source)
        all_tasks.extend(tasks)

    return all_tasks


def _load_from_source(source: SourceConfig) -> list[UserStory]:
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

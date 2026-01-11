"""PRD store - unified task format with acceptance criteria.

This module implements the Ralph pattern: aggregating tasks from all sources
into a unified prd.json file that the AI reads directly. The AI then marks
`passes: true` in this file when tasks are complete.
"""

from __future__ import annotations

import json
from datetime import datetime
from pathlib import Path

import tomllib
from pydantic import BaseModel, ConfigDict, Field

from afk.config import AFK_DIR, AfkConfig


class UserStory(BaseModel):
    """A user story in Ralph format with acceptance criteria."""

    id: str
    title: str
    description: str
    acceptance_criteria: list[str] = Field(default_factory=list)
    priority: int = 3  # 1-5, 1 = highest
    passes: bool = False
    source: str = "unknown"
    notes: str = ""

    model_config = ConfigDict(populate_by_name=True)

    def model_dump_json_compat(self) -> dict:
        """Dump to dict with camelCase keys for JSON compatibility."""
        return {
            "id": self.id,
            "title": self.title,
            "description": self.description,
            "acceptanceCriteria": self.acceptance_criteria,
            "priority": self.priority,
            "passes": self.passes,
            "source": self.source,
            "notes": self.notes,
        }

    @classmethod
    def from_json_dict(cls, data: dict) -> UserStory:
        """Create from a dict that may use camelCase keys."""
        return cls(
            id=data.get("id", ""),
            title=data.get("title") or data.get("description") or data.get("summary", ""),
            description=data.get("description") or data.get("title", ""),
            acceptance_criteria=(
                data.get("acceptanceCriteria")
                or data.get("acceptance_criteria")
                or data.get("steps")
                or []
            ),
            priority=data.get("priority", 3),
            passes=data.get("passes", False),
            source=data.get("source", "json:.afk/prd.json"),
            notes=data.get("notes", ""),
        )


class PrdDocument(BaseModel):
    """The unified PRD document."""

    project: str = ""
    branch_name: str = ""
    description: str = ""
    user_stories: list[UserStory] = Field(default_factory=list)
    last_synced: str = ""

    model_config = ConfigDict(populate_by_name=True)

    def model_dump_json_compat(self) -> dict:
        """Dump to dict with camelCase keys for JSON compatibility."""
        return {
            "project": self.project,
            "branchName": self.branch_name,
            "description": self.description,
            "userStories": [story.model_dump_json_compat() for story in self.user_stories],
            "lastSynced": self.last_synced,
        }

    @classmethod
    def from_json_dict(cls, data: dict) -> PrdDocument:
        """Create from a dict that may use camelCase keys.

        Supports multiple key names for compatibility:
        - userStories (canonical)
        - tasks (afk prd parse output)
        - items (legacy)
        """
        # Support multiple key names for stories
        story_data = data.get("userStories") or data.get("tasks") or data.get("items") or []

        stories = [UserStory.from_json_dict(item) for item in story_data]

        return cls(
            project=data.get("project", ""),
            branch_name=data.get("branchName") or data.get("branch_name", ""),
            description=data.get("description", ""),
            user_stories=stories,
            last_synced=data.get("lastSynced") or data.get("last_synced", ""),
        )


PRD_FILE = AFK_DIR / "prd.json"


def load_prd() -> PrdDocument:
    """Load the PRD document from disk."""
    if not PRD_FILE.exists():
        return PrdDocument()

    with open(PRD_FILE) as f:
        data = json.load(f)

    return PrdDocument.from_json_dict(data)


def save_prd(prd: PrdDocument) -> None:
    """Save the PRD document to disk."""
    PRD_FILE.parent.mkdir(parents=True, exist_ok=True)

    with open(PRD_FILE, "w") as f:
        json.dump(prd.model_dump_json_compat(), f, indent=2)


def sync_prd(config: AfkConfig, branch_name: str | None = None) -> PrdDocument:
    """Sync PRD from all configured sources.

    This aggregates tasks from all sources and writes them to prd.json.
    Existing completion status (passes: true) is preserved for matching IDs.

    If no sources are configured but .afk/prd.json exists with stories,
    it's used directly as the source of truth (created by afk prd parse
    or manually placed there).
    """
    from afk.sources import aggregate_tasks

    # Load existing PRD
    existing_prd = load_prd()

    # If no sources configured but PRD exists with stories, use it directly
    # This handles the case where user created .afk/prd.json via afk prd parse
    # or placed it there manually - we don't want to overwrite it
    if not config.sources and existing_prd.user_stories:
        return existing_prd

    existing_status = {story.id: story.passes for story in existing_prd.user_stories}

    # Aggregate from all sources using the sources module
    stories = aggregate_tasks(config.sources)

    # Safety check: don't wipe a populated PRD with an empty sync
    # This protects against sources returning nothing (e.g., empty beads)
    if not stories and existing_prd.user_stories:
        return existing_prd

    # Preserve completion status from previous sync
    for story in stories:
        if story.id in existing_status:
            story.passes = existing_status[story.id]

    # Sort by priority (1 = highest)
    stories.sort(key=lambda s: s.priority)

    # Get branch name
    if not branch_name:
        branch_name = _get_current_branch()

    # Build PRD document
    prd = PrdDocument(
        project=_get_project_name(),
        branch_name=branch_name,
        description=existing_prd.description or "Tasks synced from configured sources",
        user_stories=stories,
        last_synced=datetime.now().isoformat(),
    )

    save_prd(prd)
    return prd


def _get_current_branch() -> str:
    """Get current git branch name."""
    import subprocess

    try:
        result = subprocess.run(
            ["git", "branch", "--show-current"],
            capture_output=True,
            text=True,
            timeout=10,
        )
        return result.stdout.strip() or "main"
    except (FileNotFoundError, subprocess.TimeoutExpired):
        return "main"


def _get_project_name() -> str:
    """Get project name from current directory or pyproject.toml."""
    # Try pyproject.toml
    pyproject = Path("pyproject.toml")
    if pyproject.exists():
        try:
            with open(pyproject, "rb") as f:
                data = tomllib.load(f)
            name = data.get("project", {}).get("name", "")
            if not name:
                name = data.get("tool", {}).get("poetry", {}).get("name", "")
            return name if isinstance(name, str) else ""
        except Exception:
            pass

    # Fall back to directory name
    return Path.cwd().name


def get_pending_stories(prd: PrdDocument | None = None) -> list[UserStory]:
    """Get stories that haven't passed yet, sorted by priority."""
    if prd is None:
        prd = load_prd()

    pending = [story for story in prd.user_stories if not story.passes]
    pending.sort(key=lambda s: s.priority)
    return pending


def get_next_story(prd: PrdDocument | None = None) -> UserStory | None:
    """Get the next story to work on (highest priority, not passed)."""
    pending = get_pending_stories(prd)
    return pending[0] if pending else None


def mark_story_complete(story_id: str) -> bool:
    """Mark a story as complete (passes: true).

    Note: In Ralph pattern, the AI modifies prd.json directly.
    This function is for programmatic use or fallback.

    If the story came from beads, it will also be closed in beads.
    """
    prd = load_prd()

    for story in prd.user_stories:
        if story.id == story_id:
            story.passes = True
            save_prd(prd)

            # Sync completion back to source
            if story.source == "beads":
                from afk.sources.beads import close_beads_issue

                close_beads_issue(story_id)

            return True

    return False


def all_stories_complete(prd: PrdDocument | None = None) -> bool:
    """Check if all stories have passed."""
    if prd is None:
        prd = load_prd()

    if not prd.user_stories:
        return True

    return all(story.passes for story in prd.user_stories)

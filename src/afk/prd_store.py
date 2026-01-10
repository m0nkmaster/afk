"""PRD store - unified task format with acceptance criteria.

This module implements the Ralph pattern: aggregating tasks from all sources
into a unified prd.json file that the AI reads directly. The AI then marks
`passes: true` in this file when tasks are complete.
"""

from __future__ import annotations

import json
from dataclasses import asdict, dataclass, field
from datetime import datetime
from pathlib import Path
from typing import Literal

from afk.config import AFK_DIR, AfkConfig, SourceConfig


@dataclass
class UserStory:
    """A user story in Ralph format with acceptance criteria."""

    id: str
    title: str
    description: str
    acceptanceCriteria: list[str] = field(default_factory=list)
    priority: int = 3  # 1-5, 1 = highest
    passes: bool = False
    source: str = "unknown"
    notes: str = ""


@dataclass
class PrdDocument:
    """The unified PRD document."""

    project: str = ""
    branchName: str = ""
    description: str = ""
    userStories: list[UserStory] = field(default_factory=list)
    lastSynced: str = ""

    def to_dict(self) -> dict:
        """Convert to dictionary for JSON serialisation."""
        return {
            "project": self.project,
            "branchName": self.branchName,
            "description": self.description,
            "userStories": [asdict(story) for story in self.userStories],
            "lastSynced": self.lastSynced,
        }

    @classmethod
    def from_dict(cls, data: dict) -> PrdDocument:
        """Create from dictionary.
        
        Supports multiple key names for backwards compatibility:
        - userStories (canonical)
        - tasks (afk prd parse output)
        - items (legacy)
        """
        # Support multiple key names for stories
        story_data = (
            data.get("userStories")
            or data.get("tasks")
            or data.get("items")
            or []
        )
        
        stories = []
        for item in story_data:
            # Handle both field naming conventions
            story_kwargs = {
                "id": item.get("id", ""),
                "title": item.get("title") or item.get("description") or item.get("summary", ""),
                "description": item.get("description") or item.get("title", ""),
                "acceptanceCriteria": item.get("acceptanceCriteria") or item.get("steps") or [],
                "priority": item.get("priority", 3),
                "passes": item.get("passes", False),
                "source": item.get("source", "json:.afk/prd.json"),
                "notes": item.get("notes", ""),
            }
            stories.append(UserStory(**story_kwargs))
        
        return cls(
            project=data.get("project", ""),
            branchName=data.get("branchName", ""),
            description=data.get("description", ""),
            userStories=stories,
            lastSynced=data.get("lastSynced", ""),
        )


PRD_FILE = AFK_DIR / "prd.json"


def load_prd() -> PrdDocument:
    """Load the PRD document from disk."""
    if not PRD_FILE.exists():
        return PrdDocument()

    with open(PRD_FILE) as f:
        data = json.load(f)

    return PrdDocument.from_dict(data)


def save_prd(prd: PrdDocument) -> None:
    """Save the PRD document to disk."""
    PRD_FILE.parent.mkdir(parents=True, exist_ok=True)

    with open(PRD_FILE, "w") as f:
        json.dump(prd.to_dict(), f, indent=2)


def sync_prd(config: AfkConfig, branch_name: str | None = None) -> PrdDocument:
    """Sync PRD from all configured sources.

    This aggregates tasks from all sources and writes them to prd.json.
    Existing completion status (passes: true) is preserved for matching IDs.

    If no sources are configured but .afk/prd.json exists with stories,
    it's used directly as the source of truth (created by afk prd parse
    or manually placed there).
    """
    # Load existing PRD
    existing_prd = load_prd()

    # If no sources configured but PRD exists with stories, use it directly
    # This handles the case where user created .afk/prd.json via afk prd parse
    # or placed it there manually - we don't want to overwrite it
    if not config.sources and existing_prd.userStories:
        return existing_prd

    existing_status = {story.id: story.passes for story in existing_prd.userStories}

    # Aggregate from all sources
    stories: list[UserStory] = []
    for source in config.sources:
        source_stories = _load_from_source(source)
        stories.extend(source_stories)

    # Safety check: don't wipe a populated PRD with an empty sync
    # This protects against sources returning nothing (e.g., empty beads)
    if not stories and existing_prd.userStories:
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
        branchName=branch_name,
        description=existing_prd.description or "Tasks synced from configured sources",
        userStories=stories,
        lastSynced=datetime.now().isoformat(),
    )

    save_prd(prd)
    return prd


def _load_from_source(source: SourceConfig) -> list[UserStory]:
    """Load user stories from a single source."""
    if source.type == "beads":
        return _load_beads_stories()
    elif source.type == "json":
        return _load_json_stories(source.path)
    elif source.type == "markdown":
        return _load_markdown_stories(source.path)
    elif source.type == "github":
        return _load_github_stories(source.repo, source.labels)
    else:
        return []


def _load_beads_stories() -> list[UserStory]:
    """Load stories from beads."""
    import subprocess

    try:
        result = subprocess.run(
            ["bd", "ready", "--json"],
            capture_output=True,
            text=True,
            timeout=30,
        )

        if result.returncode != 0:
            return []

        data = json.loads(result.stdout)
        stories = []

        for item in data:
            task_id = item.get("id") or item.get("key") or str(item.get("number", ""))
            title = item.get("title") or item.get("summary", "")
            description = item.get("description") or item.get("body", "")

            # Extract acceptance criteria from description if present
            acceptance_criteria = _extract_acceptance_criteria(description)

            # If no AC found, use title as single criterion
            if not acceptance_criteria:
                acceptance_criteria = [f"Complete: {title}"]

            if task_id:
                stories.append(
                    UserStory(
                        id=task_id,
                        title=title,
                        description=description or title,
                        acceptanceCriteria=acceptance_criteria,
                        priority=_map_priority(item.get("priority")),
                        source="beads",
                    )
                )

        return stories

    except (FileNotFoundError, subprocess.TimeoutExpired, json.JSONDecodeError):
        return []


def _load_json_stories(path: str | None) -> list[UserStory]:
    """Load stories from JSON PRD file."""
    if not path:
        for default_path in ["prd.json", "tasks.json", ".afk/prd.json"]:
            if Path(default_path).exists():
                path = default_path
                break
        else:
            return []

    file_path = Path(path)
    if not file_path.exists():
        return []

    with open(file_path) as f:
        data = json.load(f)

    # Handle both formats
    if isinstance(data, list):
        items = data
    elif isinstance(data, dict):
        items = data.get("tasks", data.get("userStories", data.get("items", [])))
    else:
        return []

    stories = []
    for item in items:
        # Skip if already completed in source
        if item.get("passes", False):
            continue

        task_id = item.get("id") or _generate_id(item.get("title", item.get("description", "")))
        title = item.get("title") or item.get("description") or item.get("summary", "")
        description = item.get("description") or title

        # Get acceptance criteria from various field names
        acceptance_criteria = (
            item.get("acceptanceCriteria")
            or item.get("acceptance_criteria")
            or item.get("steps")
            or []
        )

        # Ensure it's a list
        if isinstance(acceptance_criteria, str):
            acceptance_criteria = [acceptance_criteria]

        # If no AC, create from description
        if not acceptance_criteria:
            acceptance_criteria = [f"Complete: {title}"]

        if task_id:
            stories.append(
                UserStory(
                    id=task_id,
                    title=title,
                    description=description,
                    acceptanceCriteria=acceptance_criteria,
                    priority=_map_priority(item.get("priority")),
                    source=f"json:{path}",
                )
            )

    return stories


def _load_markdown_stories(path: str | None) -> list[UserStory]:
    """Load stories from markdown file."""
    import re

    if not path:
        for default_path in ["tasks.md", "TODO.md", "prd.md", ".afk/tasks.md"]:
            if Path(default_path).exists():
                path = default_path
                break
        else:
            return []

    file_path = Path(path)
    if not file_path.exists():
        return []

    content = file_path.read_text()
    stories = []

    # Match markdown checkboxes
    pattern = r"^[\s]*[-*]\s*\[([ xX])\]\s*(.+)$"

    for match in re.finditer(pattern, content, re.MULTILINE):
        checked = match.group(1).lower() == "x"
        text = match.group(2).strip()

        if checked:
            continue

        task_id, description, priority = _parse_markdown_task(text)

        stories.append(
            UserStory(
                id=task_id,
                title=description,
                description=description,
                acceptanceCriteria=[f"Complete: {description}"],
                priority=priority,
                source=f"markdown:{path}",
            )
        )

    return stories


def _load_github_stories(
    repo: str | None = None,
    labels: list[str] | None = None,
) -> list[UserStory]:
    """Load stories from GitHub issues with full body."""
    import subprocess

    try:
        # Include body for acceptance criteria
        cmd = ["gh", "issue", "list", "--json", "number,title,body,labels,state"]

        if repo:
            cmd.extend(["--repo", repo])

        if labels:
            for label in labels:
                cmd.extend(["--label", label])

        cmd.extend(["--state", "open"])

        result = subprocess.run(
            cmd,
            capture_output=True,
            text=True,
            timeout=60,
        )

        if result.returncode != 0:
            return []

        issues = json.loads(result.stdout)
        stories = []

        for issue in issues:
            number = issue.get("number")
            title = issue.get("title", "")
            body = issue.get("body", "")
            issue_labels = [lbl.get("name", "") for lbl in issue.get("labels", [])]
            priority = _priority_from_labels(issue_labels)

            # Extract acceptance criteria from body
            acceptance_criteria = _extract_acceptance_criteria(body)
            if not acceptance_criteria:
                acceptance_criteria = [f"Complete: {title}"]

            if number and title:
                stories.append(
                    UserStory(
                        id=f"#{number}",
                        title=title,
                        description=body or title,
                        acceptanceCriteria=acceptance_criteria,
                        priority=priority,
                        source="github",
                    )
                )

        return stories

    except (FileNotFoundError, subprocess.TimeoutExpired, json.JSONDecodeError):
        return []


def _extract_acceptance_criteria(text: str) -> list[str]:
    """Extract acceptance criteria from text.

    Looks for common patterns:
    - Checkbox lists (- [ ] or * [ ])
    - Numbered lists under "Acceptance Criteria" heading
    - Bullet points under "AC" or "Definition of Done"
    """
    import re

    if not text:
        return []

    criteria: list[str] = []

    # Look for acceptance criteria section
    ac_patterns = [
        r"(?:acceptance\s*criteria|ac|definition\s*of\s*done|dod|requirements?)[\s:]*\n((?:[-*\d.]+\s*.+\n?)+)",
        r"##\s*(?:acceptance\s*criteria|ac|dod)\s*\n((?:[-*\d.]+\s*.+\n?)+)",
    ]

    for pattern in ac_patterns:
        match = re.search(pattern, text, re.IGNORECASE | re.MULTILINE)
        if match:
            section = match.group(1)
            # Extract list items
            for line in section.split("\n"):
                line = line.strip()
                # Remove list markers
                cleaned = re.sub(r"^[-*\d.]+\s*(\[[ x]\])?\s*", "", line)
                if cleaned:
                    criteria.append(cleaned)
            break

    # If no section found, look for checkbox items anywhere
    if not criteria:
        checkbox_pattern = r"[-*]\s*\[[ ]\]\s*(.+)"
        matches = re.findall(checkbox_pattern, text)
        criteria = [m.strip() for m in matches if m.strip()]

    return criteria


def _parse_markdown_task(text: str) -> tuple[str, str, int]:
    """Parse a markdown task line."""
    import re

    priority = 3
    description = text

    # Check for priority tag
    priority_match = re.match(r"^\[([A-Z0-9]+)\]\s*(.+)$", text)
    if priority_match:
        tag = priority_match.group(1).upper()
        description = priority_match.group(2).strip()

        if tag in ("HIGH", "CRITICAL", "URGENT", "P0", "P1"):
            priority = 1
        elif tag in ("P2", "MEDIUM"):
            priority = 2
        elif tag in ("LOW", "MINOR", "P3", "P4"):
            priority = 4

    # Check for explicit ID
    id_match = re.match(r"^([a-z0-9_-]+):\s*(.+)$", description, re.IGNORECASE)
    if id_match:
        task_id = id_match.group(1).lower()
        description = id_match.group(2).strip()
    else:
        task_id = _generate_id(description)

    return task_id, description, priority


def _generate_id(text: str) -> str:
    """Generate an ID from text."""
    clean = text[:30].lower()
    clean = "".join(c if c.isalnum() or c == " " else "" for c in clean)
    return clean.replace(" ", "-").strip("-") or "task"


def _map_priority(priority: str | int | None) -> int:
    """Map various priority formats to 1-5 scale."""
    if priority is None:
        return 3

    if isinstance(priority, int):
        # Already numeric, clamp to 1-5
        return max(1, min(5, priority))

    priority_lower = str(priority).lower()
    if priority_lower in ("high", "critical", "urgent", "1", "p0", "p1"):
        return 1
    elif priority_lower in ("medium", "normal", "2", "p2"):
        return 2
    elif priority_lower in ("low", "minor", "4", "5", "p3", "p4"):
        return 4
    else:
        return 3


def _priority_from_labels(labels: list[str]) -> int:
    """Infer priority from GitHub labels."""
    labels_lower = [lbl.lower() for lbl in labels]

    high_labels = {"priority:high", "critical", "urgent", "p0", "p1", "high-priority"}
    low_labels = {"priority:low", "minor", "p3", "p4", "low-priority", "nice-to-have"}

    if any(lbl in high_labels for lbl in labels_lower):
        return 1
    elif any(lbl in low_labels for lbl in labels_lower):
        return 4
    else:
        return 3


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
    import tomllib

    # Try pyproject.toml
    pyproject = Path("pyproject.toml")
    if pyproject.exists():
        try:
            with open(pyproject, "rb") as f:
                data = tomllib.load(f)
            return data.get("project", {}).get("name", "") or data.get("tool", {}).get("poetry", {}).get("name", "")
        except Exception:
            pass

    # Fall back to directory name
    return Path.cwd().name


def get_pending_stories(prd: PrdDocument | None = None) -> list[UserStory]:
    """Get stories that haven't passed yet, sorted by priority."""
    if prd is None:
        prd = load_prd()

    pending = [story for story in prd.userStories if not story.passes]
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

    for story in prd.userStories:
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

    if not prd.userStories:
        return True

    return all(story.passes for story in prd.userStories)

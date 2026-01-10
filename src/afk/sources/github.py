"""GitHub Issues task source adapter."""

from __future__ import annotations

import json
import subprocess
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from afk.sources import Task


def load_github_tasks(
    repo: str | None = None,
    labels: list[str] | None = None,
) -> list[Task]:
    """Load tasks from GitHub issues using gh CLI.

    Requires GitHub CLI (gh) to be installed and authenticated.
    """
    from afk.sources import Task

    try:
        cmd = ["gh", "issue", "list", "--json", "number,title,labels,state"]

        if repo:
            cmd.extend(["--repo", repo])

        if labels:
            for label in labels:
                cmd.extend(["--label", label])

        # Only open issues
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
        tasks = []

        for issue in issues:
            number = issue.get("number")
            title = issue.get("title", "")
            issue_labels = [lbl.get("name", "") for lbl in issue.get("labels", [])]
            priority = _priority_from_labels(issue_labels)

            if number and title:
                tasks.append(
                    Task(
                        id=f"#{number}",
                        description=title,
                        priority=priority,
                        source="github",
                        metadata=issue,
                    )
                )

        return tasks

    except FileNotFoundError:
        # gh CLI not installed
        return []
    except subprocess.TimeoutExpired:
        return []
    except json.JSONDecodeError:
        return []


def _priority_from_labels(labels: list[str]) -> str:
    """Infer priority from issue labels."""
    labels_lower = [lbl.lower() for lbl in labels]

    high_labels = {"priority:high", "critical", "urgent", "p0", "p1", "high-priority"}
    low_labels = {"priority:low", "minor", "p3", "p4", "low-priority", "nice-to-have"}

    if any(lbl in high_labels for lbl in labels_lower):
        return "high"
    elif any(lbl in low_labels for lbl in labels_lower):
        return "low"
    else:
        return "medium"

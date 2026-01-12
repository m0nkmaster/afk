"""GitHub Issues task source adapter."""

from __future__ import annotations

import json
import re
import subprocess

from afk.prd_store import UserStory


def load_github_tasks(
    repo: str | None = None,
    labels: list[str] | None = None,
) -> list[UserStory]:
    """Load tasks from GitHub issues using gh CLI.

    Requires GitHub CLI (gh) to be installed and authenticated.
    """
    try:
        cmd = ["gh", "issue", "list", "--json", "number,title,body,labels,state"]

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
            body = issue.get("body", "")
            issue_labels = [lbl.get("name", "") for lbl in issue.get("labels", [])]
            priority = _priority_from_labels(issue_labels)

            # Extract acceptance criteria from body
            acceptance_criteria = _extract_acceptance_criteria(body)
            if not acceptance_criteria:
                acceptance_criteria = [f"Complete: {title}"]

            if number and title:
                tasks.append(
                    UserStory(
                        id=f"#{number}",
                        title=title,
                        description=body or title,
                        acceptance_criteria=acceptance_criteria,
                        priority=priority,
                        source="github",
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


def _priority_from_labels(labels: list[str]) -> int:
    """Infer priority from issue labels."""
    labels_lower = [lbl.lower() for lbl in labels]

    high_labels = {"priority:high", "critical", "urgent", "p0", "p1", "high-priority"}
    low_labels = {"priority:low", "minor", "p3", "p4", "low-priority", "nice-to-have"}

    if any(lbl in high_labels for lbl in labels_lower):
        return 1
    elif any(lbl in low_labels for lbl in labels_lower):
        return 4
    else:
        return 3


def _extract_acceptance_criteria(text: str) -> list[str]:
    """Extract acceptance criteria from text."""
    if not text:
        return []

    criteria: list[str] = []

    # Look for acceptance criteria section
    ac_patterns = [
        r"(?:acceptance\s*criteria|ac|definition\s*of\s*done|dod|requirements?)[\s:]*\n"
        r"((?:[-*\d.]+\s*.+\n?)+)",
        r"##\s*(?:acceptance\s*criteria|ac|dod)\s*\n((?:[-*\d.]+\s*.+\n?)+)",
    ]

    for pattern in ac_patterns:
        match = re.search(pattern, text, re.IGNORECASE | re.MULTILINE)
        if match:
            section = match.group(1)
            for line in section.split("\n"):
                line = line.strip()
                cleaned = re.sub(r"^[-*\d.]+\s*(\[[ x]\])?\s*", "", line)
                if cleaned:
                    criteria.append(cleaned)
            break

    # If no section found, look for checkbox items
    if not criteria:
        checkbox_pattern = r"[-*]\s*\[[ ]\]\s*(.+)"
        matches = re.findall(checkbox_pattern, text)
        criteria = [m.strip() for m in matches if m.strip()]

    return criteria

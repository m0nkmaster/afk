"""Git operations for afk."""

from __future__ import annotations

import shutil
import subprocess
from datetime import datetime
from pathlib import Path

from afk.config import PROGRESS_FILE, AfkConfig


def is_git_repo() -> bool:
    """Check if current directory is a git repository."""
    result = subprocess.run(
        ["git", "rev-parse", "--git-dir"],
        capture_output=True,
        text=True,
    )
    return result.returncode == 0


def get_current_branch() -> str | None:
    """Get the current git branch name."""
    result = subprocess.run(
        ["git", "rev-parse", "--abbrev-ref", "HEAD"],
        capture_output=True,
        text=True,
    )
    if result.returncode == 0:
        return result.stdout.strip()
    return None


def create_branch(branch_name: str, config: AfkConfig) -> bool:
    """Create and checkout a new branch.

    Args:
        branch_name: Name of the branch (will be prefixed)
        config: afk configuration

    Returns:
        True if branch created/checked out successfully
    """
    if not config.git.auto_branch:
        return False

    full_branch = f"{config.git.branch_prefix}{branch_name}"

    # Check if branch exists
    result = subprocess.run(
        ["git", "rev-parse", "--verify", full_branch],
        capture_output=True,
        text=True,
    )

    if result.returncode == 0:
        # Branch exists, checkout
        result = subprocess.run(
            ["git", "checkout", full_branch],
            capture_output=True,
            text=True,
        )
    else:
        # Create new branch
        result = subprocess.run(
            ["git", "checkout", "-b", full_branch],
            capture_output=True,
            text=True,
        )

    return result.returncode == 0


def auto_commit(task_id: str, message: str, config: AfkConfig) -> bool:
    """Commit changes with auto-generated message.

    Args:
        task_id: ID of the completed task
        message: Commit message (or task description)
        config: afk configuration

    Returns:
        True if commit successful
    """
    if not config.git.auto_commit:
        return False

    # Stage all changes
    stage_result = subprocess.run(
        ["git", "add", "-A"],
        capture_output=True,
        text=True,
    )
    if stage_result.returncode != 0:
        return False

    # Check if there are changes to commit
    status_result = subprocess.run(
        ["git", "status", "--porcelain"],
        capture_output=True,
        text=True,
    )
    if not status_result.stdout.strip():
        # Nothing to commit
        return True

    # Format commit message
    commit_msg = config.git.commit_message_template.format(
        task_id=task_id,
        message=message[:50] if len(message) > 50 else message,
    )

    # Commit
    commit_result = subprocess.run(
        ["git", "commit", "-m", commit_msg],
        capture_output=True,
        text=True,
    )

    return commit_result.returncode == 0


def get_staged_files() -> list[str]:
    """Get list of staged files."""
    result = subprocess.run(
        ["git", "diff", "--cached", "--name-only"],
        capture_output=True,
        text=True,
    )
    if result.returncode == 0:
        return result.stdout.strip().split("\n")
    return []


def get_uncommitted_changes() -> bool:
    """Check if there are uncommitted changes."""
    result = subprocess.run(
        ["git", "status", "--porcelain"],
        capture_output=True,
        text=True,
    )
    return bool(result.stdout.strip())


def archive_session(config: AfkConfig, reason: str = "manual") -> Path | None:
    """Archive current session files.

    Archives progress.json and prompt.md to a timestamped directory.

    Args:
        config: afk configuration
        reason: Reason for archiving (e.g., 'branch_change', 'complete', 'manual')

    Returns:
        Path to archive directory, or None if archiving disabled/failed
    """
    if not config.archive.enabled:
        return None

    # Create archive directory with timestamp
    timestamp = datetime.now().strftime("%Y-%m-%d_%H-%M-%S")
    branch = get_current_branch() or "unknown"
    # Sanitise branch name for filesystem
    safe_branch = branch.replace("/", "-").replace("\\", "-")

    archive_name = f"{timestamp}_{safe_branch}_{reason}"
    archive_path = Path(config.archive.directory) / archive_name

    archive_path.mkdir(parents=True, exist_ok=True)

    # Copy progress file if exists
    if PROGRESS_FILE.exists():
        shutil.copy2(PROGRESS_FILE, archive_path / "progress.json")

    # Copy prompt file if exists
    prompt_path = Path(config.output.file_path)
    if prompt_path.exists():
        shutil.copy2(prompt_path, archive_path / "prompt.md")

    # Write metadata
    metadata = {
        "archived_at": datetime.now().isoformat(),
        "branch": branch,
        "reason": reason,
    }
    import json

    with open(archive_path / "metadata.json", "w") as f:
        json.dump(metadata, f, indent=2)

    return archive_path


def clear_session() -> None:
    """Clear current session progress for fresh start."""
    if PROGRESS_FILE.exists():
        PROGRESS_FILE.unlink()


def should_archive_on_branch_change(new_branch: str | None, config: AfkConfig) -> bool:
    """Check if we should archive when changing branches.

    Args:
        new_branch: The branch we're switching to
        config: afk configuration

    Returns:
        True if we should archive the current session
    """
    if not config.archive.on_branch_change:
        return False

    if not PROGRESS_FILE.exists():
        return False

    current_branch = get_current_branch()
    if current_branch is None:
        return False

    # Different branch = archive
    return new_branch != current_branch

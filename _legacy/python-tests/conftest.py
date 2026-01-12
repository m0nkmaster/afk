"""Shared fixtures for afk tests."""

from __future__ import annotations

import json
import os
from collections.abc import Generator
from pathlib import Path
from typing import TYPE_CHECKING
from unittest.mock import MagicMock

import pytest

if TYPE_CHECKING:
    from afk.config import AfkConfig


@pytest.fixture
def temp_project(tmp_path: Path) -> Generator[Path, None, None]:
    """Create a temporary project directory and change to it."""
    original_dir = os.getcwd()
    os.chdir(tmp_path)
    try:
        yield tmp_path
    finally:
        os.chdir(original_dir)


@pytest.fixture
def temp_afk_dir(temp_project: Path) -> Path:
    """Create a temporary .afk directory."""
    afk_dir = temp_project / ".afk"
    afk_dir.mkdir()
    return afk_dir


@pytest.fixture
def sample_config_data() -> dict:
    """Sample configuration data."""
    return {
        "sources": [
            {"type": "beads"},
            {"type": "json", "path": "tasks.json"},
        ],
        "feedback_loops": {
            "lint": "ruff check .",
            "types": "mypy .",
            "test": "pytest",
        },
        "limits": {
            "max_iterations": 10,
            "max_task_failures": 2,
            "timeout_minutes": 60,
        },
        "output": {"default": "clipboard", "file_path": ".afk/prompt.md"},
        "ai_cli": {"command": "claude", "args": ["-p"]},
        "prompt": {
            "template": "default",
            "context_files": ["AGENTS.md", "README.md"],
            "instructions": ["Always run tests"],
        },
    }


@pytest.fixture
def sample_config(temp_afk_dir: Path, sample_config_data: dict) -> AfkConfig:
    """Create a sample config file and return loaded config."""
    from afk.config import AfkConfig

    config_path = temp_afk_dir / "config.json"
    with open(config_path, "w") as f:
        json.dump(sample_config_data, f)

    return AfkConfig.load(config_path)


@pytest.fixture
def sample_progress_data() -> dict:
    """Sample progress data."""
    return {
        "started_at": "2025-01-10T10:00:00",
        "iterations": 3,
        "tasks": {
            "task-1": {
                "id": "task-1",
                "source": "beads",
                "status": "completed",
                "started_at": "2025-01-10T10:05:00",
                "completed_at": "2025-01-10T10:15:00",
                "failure_count": 0,
                "commits": ["abc123"],
                "message": "Done",
            },
            "task-2": {
                "id": "task-2",
                "source": "json:tasks.json",
                "status": "in_progress",
                "started_at": "2025-01-10T10:20:00",
                "failure_count": 1,
            },
        },
    }


@pytest.fixture
def sample_tasks_json(temp_project: Path) -> Path:
    """Create a sample tasks.json file."""
    tasks_data = {
        "tasks": [
            {"id": "task-1", "description": "Implement feature A", "priority": "high"},
            {"id": "task-2", "description": "Fix bug B", "priority": "medium"},
            {"id": "task-3", "description": "Refactor C", "priority": "low", "passes": True},
        ]
    }
    tasks_path = temp_project / "tasks.json"
    with open(tasks_path, "w") as f:
        json.dump(tasks_data, f)
    return tasks_path


@pytest.fixture
def sample_tasks_md(temp_project: Path) -> Path:
    """Create a sample tasks.md file."""
    content = """\
# Tasks

- [ ] [HIGH] task-1: Implement the feature
- [ ] task-2: Fix the bug
- [x] task-3: Already done
- [ ] [LOW] Do something minor
"""
    tasks_path = temp_project / "tasks.md"
    tasks_path.write_text(content)
    return tasks_path


@pytest.fixture
def mock_subprocess_run(monkeypatch: pytest.MonkeyPatch) -> MagicMock:
    """Mock subprocess.run for testing external commands."""
    mock = MagicMock()
    monkeypatch.setattr("subprocess.run", mock)
    return mock


@pytest.fixture
def python_project(temp_project: Path) -> Path:
    """Create a Python project structure."""
    # pyproject.toml
    (temp_project / "pyproject.toml").write_text('[project]\nname = "test-project"\n')
    # README.md
    (temp_project / "README.md").write_text("# Test Project\n")
    # AGENTS.md
    (temp_project / "AGENTS.md").write_text("# Agent Instructions\n")
    return temp_project


@pytest.fixture
def node_project(temp_project: Path) -> Path:
    """Create a Node.js project structure."""
    (temp_project / "package.json").write_text('{"name": "test"}\n')
    (temp_project / "README.md").write_text("# Test Project\n")
    return temp_project

"""Tests for afk.sources.github module."""

from __future__ import annotations

import json
import subprocess
from unittest.mock import MagicMock, patch

from afk.sources.github import (
    _priority_from_labels,
    load_github_tasks,
)


class TestPriorityFromLabels:
    """Tests for _priority_from_labels function."""

    def test_empty_labels(self) -> None:
        """Test empty labels returns 3 (medium)."""
        assert _priority_from_labels([]) == 3

    def test_high_priority_labels(self) -> None:
        """Test high priority labels return 1."""
        assert _priority_from_labels(["priority:high"]) == 1
        assert _priority_from_labels(["critical"]) == 1
        assert _priority_from_labels(["urgent"]) == 1
        assert _priority_from_labels(["p0"]) == 1
        assert _priority_from_labels(["p1"]) == 1
        assert _priority_from_labels(["high-priority"]) == 1

    def test_low_priority_labels(self) -> None:
        """Test low priority labels return 4."""
        assert _priority_from_labels(["priority:low"]) == 4
        assert _priority_from_labels(["minor"]) == 4
        assert _priority_from_labels(["p3"]) == 4
        assert _priority_from_labels(["p4"]) == 4
        assert _priority_from_labels(["low-priority"]) == 4
        assert _priority_from_labels(["nice-to-have"]) == 4

    def test_unknown_labels(self) -> None:
        """Test unknown labels return 3 (medium)."""
        assert _priority_from_labels(["bug"]) == 3
        assert _priority_from_labels(["enhancement"]) == 3

    def test_case_insensitive(self) -> None:
        """Test label matching is case insensitive."""
        assert _priority_from_labels(["CRITICAL"]) == 1
        assert _priority_from_labels(["Priority:High"]) == 1

    def test_multiple_labels(self) -> None:
        """Test with multiple labels, high takes precedence."""
        assert _priority_from_labels(["bug", "critical", "p3"]) == 1


class TestLoadGithubTasks:
    """Tests for load_github_tasks function."""

    def test_gh_not_installed(self) -> None:
        """Test when gh CLI is not installed."""
        with patch("subprocess.run") as mock_run:
            mock_run.side_effect = FileNotFoundError()
            tasks = load_github_tasks()
            assert tasks == []

    def test_timeout(self) -> None:
        """Test when command times out."""
        with patch("subprocess.run") as mock_run:
            mock_run.side_effect = subprocess.TimeoutExpired(cmd="gh", timeout=60)
            tasks = load_github_tasks()
            assert tasks == []

    def test_non_zero_return(self) -> None:
        """Test when command returns non-zero."""
        with patch("subprocess.run") as mock_run:
            mock_run.return_value = MagicMock(returncode=1, stdout="", stderr="error")
            tasks = load_github_tasks()
            assert tasks == []

    def test_json_decode_error(self) -> None:
        """Test when output is not valid JSON."""
        with patch("subprocess.run") as mock_run:
            mock_run.return_value = MagicMock(returncode=0, stdout="not json")
            tasks = load_github_tasks()
            assert tasks == []

    def test_successful_load(self) -> None:
        """Test successful load of GitHub issues."""
        issues = [
            {
                "number": 1,
                "title": "First issue",
                "body": "Issue body",
                "labels": [{"name": "bug"}],
                "state": "open",
            },
            {
                "number": 2,
                "title": "Second issue",
                "body": "",
                "labels": [{"name": "critical"}],
                "state": "open",
            },
        ]
        with patch("subprocess.run") as mock_run:
            mock_run.return_value = MagicMock(
                returncode=0,
                stdout=json.dumps(issues),
            )
            tasks = load_github_tasks()
            assert len(tasks) == 2
            assert tasks[0].id == "#1"
            assert tasks[0].title == "First issue"
            assert tasks[0].priority == 3  # Medium (bug label)
            assert tasks[0].source == "github"
            assert tasks[1].priority == 1  # High (critical label)

    def test_with_repo_option(self) -> None:
        """Test loading with specific repo."""
        with patch("subprocess.run") as mock_run:
            mock_run.return_value = MagicMock(returncode=0, stdout="[]")
            load_github_tasks(repo="owner/repo")

            # Verify --repo was passed
            call_args = mock_run.call_args[0][0]
            assert "--repo" in call_args
            assert "owner/repo" in call_args

    def test_with_labels_option(self) -> None:
        """Test loading with label filters."""
        with patch("subprocess.run") as mock_run:
            mock_run.return_value = MagicMock(returncode=0, stdout="[]")
            load_github_tasks(labels=["bug", "help-wanted"])

            # Verify --label was passed for each label
            call_args = mock_run.call_args[0][0]
            label_indices = [i for i, arg in enumerate(call_args) if arg == "--label"]
            assert len(label_indices) == 2

    def test_skips_issues_without_number(self) -> None:
        """Test that issues without number are skipped."""
        issues = [
            {"title": "No number", "labels": []},
            {"number": 1, "title": "Has number", "labels": []},
        ]
        with patch("subprocess.run") as mock_run:
            mock_run.return_value = MagicMock(
                returncode=0,
                stdout=json.dumps(issues),
            )
            tasks = load_github_tasks()
            assert len(tasks) == 1
            assert tasks[0].id == "#1"

    def test_skips_issues_without_title(self) -> None:
        """Test that issues without title are skipped."""
        issues = [
            {"number": 1, "labels": []},
            {"number": 2, "title": "Has title", "labels": []},
        ]
        with patch("subprocess.run") as mock_run:
            mock_run.return_value = MagicMock(
                returncode=0,
                stdout=json.dumps(issues),
            )
            tasks = load_github_tasks()
            assert len(tasks) == 1
            assert tasks[0].title == "Has title"

    def test_acceptance_criteria_extracted(self) -> None:
        """Test that acceptance criteria are extracted from body."""
        issues = [
            {
                "number": 1,
                "title": "Issue with AC",
                "body": "Description\n\nAcceptance Criteria:\n- Step 1\n- Step 2",
                "labels": [],
            },
        ]
        with patch("subprocess.run") as mock_run:
            mock_run.return_value = MagicMock(
                returncode=0,
                stdout=json.dumps(issues),
            )
            tasks = load_github_tasks()
            assert len(tasks[0].acceptance_criteria) >= 1

    def test_command_structure(self) -> None:
        """Test the gh command is structured correctly."""
        with patch("subprocess.run") as mock_run:
            mock_run.return_value = MagicMock(returncode=0, stdout="[]")
            load_github_tasks()

            call_args = mock_run.call_args[0][0]
            assert call_args[:4] == ["gh", "issue", "list", "--json"]
            assert "--state" in call_args
            assert "open" in call_args

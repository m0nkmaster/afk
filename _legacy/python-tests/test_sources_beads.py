"""Tests for afk.sources.beads module."""

from __future__ import annotations

import json
import subprocess
from unittest.mock import MagicMock, patch

from afk.sources.beads import (
    _map_beads_priority,
    _parse_beads_text_output,
    close_beads_issue,
    load_beads_tasks,
)


class TestMapBeadsPriority:
    """Tests for _map_beads_priority function."""

    def test_none_priority(self) -> None:
        """Test None returns 3 (medium)."""
        assert _map_beads_priority(None) == 3

    def test_integer_high(self) -> None:
        """Test low integers map to high priority."""
        assert _map_beads_priority(0) == 1  # Clamped to 1
        assert _map_beads_priority(1) == 1

    def test_integer_medium(self) -> None:
        """Test mid integers pass through."""
        assert _map_beads_priority(2) == 2
        assert _map_beads_priority(3) == 3

    def test_integer_low(self) -> None:
        """Test high integers are clamped."""
        assert _map_beads_priority(4) == 4
        assert _map_beads_priority(10) == 5  # Clamped to 5

    def test_string_high(self) -> None:
        """Test high priority strings map to 1."""
        assert _map_beads_priority("high") == 1
        assert _map_beads_priority("critical") == 1
        assert _map_beads_priority("urgent") == 1
        assert _map_beads_priority("p0") == 1
        assert _map_beads_priority("P1") == 1

    def test_string_low(self) -> None:
        """Test low priority strings map to 4."""
        assert _map_beads_priority("low") == 4
        assert _map_beads_priority("minor") == 4
        assert _map_beads_priority("p3") == 4
        assert _map_beads_priority("P4") == 4

    def test_string_medium(self) -> None:
        """Test medium priority strings map to 3."""
        assert _map_beads_priority("medium") == 3
        assert _map_beads_priority("normal") == 3
        assert _map_beads_priority("p2") == 3


class TestLoadBeadsTasks:
    """Tests for load_beads_tasks function."""

    def test_bd_not_installed(self) -> None:
        """Test when bd command is not installed."""
        with patch("subprocess.run") as mock_run:
            mock_run.side_effect = FileNotFoundError()
            tasks = load_beads_tasks()
            assert tasks == []

    def test_timeout(self) -> None:
        """Test when command times out."""
        with patch("subprocess.run") as mock_run:
            mock_run.side_effect = subprocess.TimeoutExpired(cmd="bd", timeout=30)
            tasks = load_beads_tasks()
            assert tasks == []

    def test_json_output(self) -> None:
        """Test parsing JSON output from bd ready --json."""
        json_data = [
            {"id": "issue-1", "title": "First issue", "priority": "high"},
            {"id": "issue-2", "title": "Second issue"},
        ]
        with patch("subprocess.run") as mock_run:
            mock_run.return_value = MagicMock(
                returncode=0,
                stdout=json.dumps(json_data),
            )
            tasks = load_beads_tasks()
            assert len(tasks) == 2
            assert tasks[0].id == "issue-1"
            assert tasks[0].title == "First issue"
            assert tasks[0].priority == 1  # "high" maps to 1
            assert tasks[0].source == "beads"
            assert tasks[1].priority == 3  # Default

    def test_json_output_alternative_fields(self) -> None:
        """Test parsing JSON with alternative field names."""
        json_data = [
            {"key": "ISSUE-123", "description": "Issue description", "priority": 1},
            {"number": 456, "summary": "Summary text"},
        ]
        with patch("subprocess.run") as mock_run:
            mock_run.return_value = MagicMock(
                returncode=0,
                stdout=json.dumps(json_data),
            )
            tasks = load_beads_tasks()
            assert len(tasks) == 2
            assert tasks[0].id == "ISSUE-123"
            assert tasks[1].id == "456"

    def test_json_decode_error_fallback(self) -> None:
        """Test fallback to text parsing on JSON error."""
        with patch("subprocess.run") as mock_run:
            # First call (--json) returns invalid JSON
            # Second call (no --json) returns text
            mock_run.side_effect = [
                MagicMock(returncode=0, stdout="not valid json"),
                MagicMock(returncode=0, stdout="task-1: Description\n"),
            ]
            tasks = load_beads_tasks()
            assert len(tasks) == 1
            assert tasks[0].id == "task-1"

    def test_non_zero_return_fallback(self) -> None:
        """Test fallback to text parsing on non-zero return."""
        with patch("subprocess.run") as mock_run:
            # First call (--json) fails
            # Second call (no --json) succeeds
            mock_run.side_effect = [
                MagicMock(returncode=1, stdout="", stderr="error"),
                MagicMock(returncode=0, stdout="task-1: Description\n"),
            ]
            tasks = load_beads_tasks()
            assert len(tasks) == 1

    def test_skips_empty_id_tasks(self) -> None:
        """Test that tasks without id are skipped."""
        json_data = [
            {"id": "", "title": "No ID"},
            {"id": "good", "title": "Good task"},
        ]
        with patch("subprocess.run") as mock_run:
            mock_run.return_value = MagicMock(
                returncode=0,
                stdout=json.dumps(json_data),
            )
            tasks = load_beads_tasks()
            assert len(tasks) == 1
            assert tasks[0].id == "good"


class TestParseBeadsTextOutput:
    """Tests for _parse_beads_text_output function."""

    def test_bd_not_installed(self) -> None:
        """Test when bd is not installed."""
        with patch("subprocess.run") as mock_run:
            mock_run.side_effect = FileNotFoundError()
            tasks = _parse_beads_text_output()
            assert tasks == []

    def test_timeout(self) -> None:
        """Test when command times out."""
        with patch("subprocess.run") as mock_run:
            mock_run.side_effect = subprocess.TimeoutExpired(cmd="bd", timeout=30)
            tasks = _parse_beads_text_output()
            assert tasks == []

    def test_non_zero_return(self) -> None:
        """Test when command returns non-zero."""
        with patch("subprocess.run") as mock_run:
            mock_run.return_value = MagicMock(returncode=1, stdout="")
            tasks = _parse_beads_text_output()
            assert tasks == []

    def test_colon_format(self) -> None:
        """Test parsing 'ID: description' format."""
        with patch("subprocess.run") as mock_run:
            mock_run.return_value = MagicMock(
                returncode=0,
                stdout="task-1: First task\ntask-2: Second task\n",
            )
            tasks = _parse_beads_text_output()
            assert len(tasks) == 2
            assert tasks[0].id == "task-1"
            assert tasks[0].title == "First task"
            assert tasks[1].id == "task-2"

    def test_no_colon_format(self) -> None:
        """Test parsing lines without colons."""
        with patch("subprocess.run") as mock_run:
            mock_run.return_value = MagicMock(
                returncode=0,
                stdout="Implement feature\nFix bug\n",
            )
            tasks = _parse_beads_text_output()
            assert len(tasks) == 2
            assert tasks[0].title == "Implement feature"
            # ID should be generated from description
            assert tasks[0].id == "implement-feature"

    def test_empty_lines_skipped(self) -> None:
        """Test that empty lines are skipped."""
        with patch("subprocess.run") as mock_run:
            mock_run.return_value = MagicMock(
                returncode=0,
                stdout="task-1: First\n\n\ntask-2: Second\n",
            )
            tasks = _parse_beads_text_output()
            assert len(tasks) == 2

    def test_all_tasks_medium_priority(self) -> None:
        """Test that text parsed tasks default to priority 3 (medium)."""
        with patch("subprocess.run") as mock_run:
            mock_run.return_value = MagicMock(
                returncode=0,
                stdout="task-1: Description\n",
            )
            tasks = _parse_beads_text_output()
            assert tasks[0].priority == 3


class TestCloseBeadsIssue:
    """Tests for close_beads_issue function."""

    def test_close_success(self) -> None:
        """Test successful close returns True."""
        with patch("subprocess.run") as mock_run:
            mock_run.return_value = MagicMock(returncode=0)
            result = close_beads_issue("issue-123")
            assert result is True
            mock_run.assert_called_once_with(
                ["bd", "close", "issue-123"],
                capture_output=True,
                text=True,
                timeout=30,
            )

    def test_close_failure(self) -> None:
        """Test failed close returns False."""
        with patch("subprocess.run") as mock_run:
            mock_run.return_value = MagicMock(returncode=1)
            result = close_beads_issue("issue-123")
            assert result is False

    def test_close_bd_not_installed(self) -> None:
        """Test close when bd not installed returns False."""
        with patch("subprocess.run") as mock_run:
            mock_run.side_effect = FileNotFoundError()
            result = close_beads_issue("issue-123")
            assert result is False

    def test_close_timeout(self) -> None:
        """Test close on timeout returns False."""
        with patch("subprocess.run") as mock_run:
            mock_run.side_effect = subprocess.TimeoutExpired("bd", 30)
            result = close_beads_issue("issue-123")
            assert result is False

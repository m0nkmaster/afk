"""Tests for afk.sources.beads module."""

from __future__ import annotations

import json
import subprocess
from unittest.mock import MagicMock, patch

from afk.sources.beads import (
    _map_beads_priority,
    _parse_beads_text_output,
    load_beads_tasks,
)


class TestMapBeadsPriority:
    """Tests for _map_beads_priority function."""

    def test_none_priority(self) -> None:
        """Test None returns medium."""
        assert _map_beads_priority(None) == "medium"

    def test_integer_high(self) -> None:
        """Test low integers map to high."""
        assert _map_beads_priority(0) == "high"
        assert _map_beads_priority(1) == "high"

    def test_integer_medium(self) -> None:
        """Test mid integers map to medium."""
        assert _map_beads_priority(2) == "medium"
        assert _map_beads_priority(3) == "medium"

    def test_integer_low(self) -> None:
        """Test high integers map to low."""
        assert _map_beads_priority(4) == "low"
        assert _map_beads_priority(10) == "low"

    def test_string_high(self) -> None:
        """Test high priority strings."""
        assert _map_beads_priority("high") == "high"
        assert _map_beads_priority("critical") == "high"
        assert _map_beads_priority("urgent") == "high"
        assert _map_beads_priority("p0") == "high"
        assert _map_beads_priority("P1") == "high"

    def test_string_low(self) -> None:
        """Test low priority strings."""
        assert _map_beads_priority("low") == "low"
        assert _map_beads_priority("minor") == "low"
        assert _map_beads_priority("p3") == "low"
        assert _map_beads_priority("P4") == "low"

    def test_string_medium(self) -> None:
        """Test medium priority strings."""
        assert _map_beads_priority("medium") == "medium"
        assert _map_beads_priority("normal") == "medium"
        assert _map_beads_priority("p2") == "medium"


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
            assert tasks[0].description == "First issue"
            assert tasks[0].priority == "high"
            assert tasks[0].source == "beads"
            assert tasks[1].priority == "medium"

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
            assert tasks[0].description == "Issue description"
            assert tasks[1].id == "456"
            assert tasks[1].description == "Summary text"

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

    def test_skips_empty_tasks(self) -> None:
        """Test that tasks without id or description are skipped."""
        json_data = [
            {"id": "", "title": "No ID"},
            {"id": "valid", "title": ""},
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
            assert tasks[0].description == "First task"
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
            assert tasks[0].description == "Implement feature"
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
        """Test that text parsed tasks default to medium priority."""
        with patch("subprocess.run") as mock_run:
            mock_run.return_value = MagicMock(
                returncode=0,
                stdout="task-1: Description\n",
            )
            tasks = _parse_beads_text_output()
            assert tasks[0].priority == "medium"

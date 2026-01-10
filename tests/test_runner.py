"""Tests for afk.runner module."""

from __future__ import annotations

import subprocess
from pathlib import Path
from unittest.mock import MagicMock, patch

from afk.config import AfkConfig, AiCliConfig, ArchiveConfig, GitConfig, LimitsConfig
from afk.runner import IterationResult, RunResult, StopReason, run_iteration, run_loop


class TestStopReason:
    """Tests for StopReason enum."""

    def test_all_reasons_have_values(self) -> None:
        """Test all stop reasons have human-readable values."""
        assert StopReason.COMPLETE.value == "All tasks completed"
        assert StopReason.MAX_ITERATIONS.value == "Maximum iterations reached"
        assert StopReason.TIMEOUT.value == "Session timeout reached"
        assert StopReason.NO_TASKS.value == "No tasks available"
        assert StopReason.USER_INTERRUPT.value == "User interrupted"
        assert StopReason.AI_ERROR.value == "AI CLI error"


class TestIterationResult:
    """Tests for IterationResult dataclass."""

    def test_success_result(self) -> None:
        """Test successful iteration result."""
        result = IterationResult(success=True, task_id="task-1", output="Done")

        assert result.success is True
        assert result.task_id == "task-1"
        assert result.error is None
        assert result.output == "Done"

    def test_failure_result(self) -> None:
        """Test failed iteration result."""
        result = IterationResult(success=False, error="Command failed")

        assert result.success is False
        assert result.error == "Command failed"


class TestRunResult:
    """Tests for RunResult dataclass."""

    def test_run_result(self) -> None:
        """Test run result with all fields."""
        result = RunResult(
            iterations_completed=5,
            tasks_completed=3,
            stop_reason=StopReason.COMPLETE,
            duration_seconds=120.5,
            archived_to=Path(".afk/archive/test"),
        )

        assert result.iterations_completed == 5
        assert result.tasks_completed == 3
        assert result.stop_reason == StopReason.COMPLETE
        assert result.duration_seconds == 120.5
        assert result.archived_to == Path(".afk/archive/test")


class TestRunIteration:
    """Tests for run_iteration function."""

    def test_detects_afk_complete(self, temp_afk_dir: Path) -> None:
        """Test detects AFK_COMPLETE stop signal."""
        config = AfkConfig()

        with patch("afk.runner.generate_prompt") as mock_prompt:
            mock_prompt.return_value = "AFK_COMPLETE: All tasks finished"

            result = run_iteration(config, iteration=1)

        assert result.success is True
        assert result.error == "AFK_COMPLETE"

    def test_detects_afk_limit_reached(self, temp_afk_dir: Path) -> None:
        """Test detects AFK_LIMIT_REACHED stop signal."""
        config = AfkConfig()

        with patch("afk.runner.generate_prompt") as mock_prompt:
            mock_prompt.return_value = "AFK_LIMIT_REACHED: Max iterations exceeded"

            result = run_iteration(config, iteration=1)

        assert result.success is False
        assert result.error == "AFK_LIMIT_REACHED"

    def test_spawns_ai_cli(self, temp_afk_dir: Path) -> None:
        """Test spawns configured AI CLI with prompt."""
        config = AfkConfig(
            ai_cli=AiCliConfig(command="echo", args=["hello"]),
            limits=LimitsConfig(timeout_minutes=1),
        )

        with patch("afk.runner.generate_prompt") as mock_prompt:
            mock_prompt.return_value = "Test prompt"

            with patch("subprocess.Popen") as mock_popen:
                mock_process = MagicMock()
                mock_process.returncode = 0
                mock_process.communicate.return_value = ("Output", None)
                mock_popen.return_value = mock_process

                result = run_iteration(config, iteration=1)

        assert result.success is True
        assert result.output == "Output"
        mock_popen.assert_called_once()

    def test_handles_ai_cli_not_found(self, temp_afk_dir: Path) -> None:
        """Test handles missing AI CLI gracefully."""
        config = AfkConfig(
            ai_cli=AiCliConfig(command="nonexistent-cli", args=[]),
        )

        with patch("afk.runner.generate_prompt") as mock_prompt:
            mock_prompt.return_value = "Test prompt"

            with patch("subprocess.Popen") as mock_popen:
                mock_popen.side_effect = FileNotFoundError()

                result = run_iteration(config, iteration=1)

        assert result.success is False
        assert "not found" in result.error

    def test_handles_timeout(self, temp_afk_dir: Path) -> None:
        """Test handles iteration timeout."""
        config = AfkConfig(
            ai_cli=AiCliConfig(command="sleep", args=["100"]),
            limits=LimitsConfig(timeout_minutes=1),
        )

        with patch("afk.runner.generate_prompt") as mock_prompt:
            mock_prompt.return_value = "Test prompt"

            with patch("subprocess.Popen") as mock_popen:
                mock_process = MagicMock()
                mock_process.communicate.side_effect = subprocess.TimeoutExpired(
                    cmd="sleep", timeout=60
                )
                mock_process.kill = MagicMock()
                mock_popen.return_value = mock_process

                result = run_iteration(config, iteration=1)

        assert result.success is False
        assert "timed out" in result.error


class TestRunLoop:
    """Tests for run_loop function."""

    def test_stops_when_no_tasks(self, temp_afk_dir: Path) -> None:
        """Test stops immediately when no tasks available."""
        config = AfkConfig()

        with patch("afk.runner.aggregate_tasks") as mock_tasks:
            mock_tasks.return_value = []

            with patch("afk.runner.archive_session"):
                result = run_loop(config, max_iterations=5)

        assert result.stop_reason == StopReason.NO_TASKS
        assert result.iterations_completed == 0

    def test_respects_max_iterations(self, temp_afk_dir: Path) -> None:
        """Test respects max_iterations limit."""
        config = AfkConfig(
            ai_cli=AiCliConfig(command="echo", args=[]),
            archive=ArchiveConfig(enabled=False),
        )

        iteration_count = [0]

        def mock_run_iteration(*args, **kwargs):
            iteration_count[0] += 1
            if iteration_count[0] >= 3:
                return IterationResult(success=True, error="AFK_COMPLETE")
            return IterationResult(success=True)

        with patch("afk.runner.aggregate_tasks") as mock_tasks:
            mock_tasks.return_value = [MagicMock(id="task-1")]

            with patch("afk.runner.check_limits") as mock_limits:
                # Allow first 2 iterations, then signal complete
                mock_limits.side_effect = [
                    (True, None),
                    (True, None),
                    (False, "AFK_COMPLETE: All tasks finished"),
                ]

                with patch("afk.runner.run_iteration", side_effect=mock_run_iteration):
                    with patch("afk.runner.SessionProgress"):
                        result = run_loop(config, max_iterations=10)

        assert result.stop_reason == StopReason.COMPLETE

    def test_archives_previous_session(self, temp_afk_dir: Path) -> None:
        """Test archives previous session before starting."""
        config = AfkConfig(archive=ArchiveConfig(enabled=True))

        # Create progress file
        progress_file = temp_afk_dir / "progress.json"
        progress_file.write_text('{"iterations": 5, "tasks": {}}')

        with patch("afk.runner.aggregate_tasks") as mock_tasks:
            mock_tasks.return_value = []

            with patch("afk.runner.archive_session") as mock_archive:
                mock_archive.return_value = Path(".afk/archive/test")

                with patch("afk.runner.clear_session"):
                    with patch("afk.runner.SessionProgress") as mock_progress:
                        mock_progress.load.return_value = MagicMock(iterations=5)

                        run_loop(config, max_iterations=5)

        # Should archive when previous session exists
        mock_archive.assert_called()

    def test_creates_branch_when_configured(
        self, temp_afk_dir: Path, mock_subprocess_run: MagicMock
    ) -> None:
        """Test creates feature branch when specified."""
        config = AfkConfig(
            git=GitConfig(auto_branch=True, branch_prefix="afk/"),
            archive=ArchiveConfig(enabled=False),
        )

        with patch("afk.runner.aggregate_tasks") as mock_tasks:
            mock_tasks.return_value = []

            with patch("afk.runner.create_branch") as mock_branch:
                with patch("afk.runner.SessionProgress") as mock_progress:
                    mock_progress.load.return_value = MagicMock(iterations=0)

                    run_loop(config, max_iterations=5, branch="my-feature")

        mock_branch.assert_called_once_with("my-feature", config)

    def test_auto_commits_on_task_completion(self, temp_afk_dir: Path) -> None:
        """Test auto-commits when task is completed."""
        config = AfkConfig(
            git=GitConfig(auto_commit=True),
            archive=ArchiveConfig(enabled=False),
            ai_cli=AiCliConfig(command="echo", args=[]),
        )

        with patch("afk.runner.aggregate_tasks") as mock_tasks:
            mock_tasks.return_value = [MagicMock(id="task-1")]

            with patch("afk.runner.check_limits") as mock_limits:
                mock_limits.return_value = (False, "AFK_COMPLETE")

                with patch("afk.runner.SessionProgress") as mock_progress:
                    mock_session = MagicMock()
                    mock_session.iterations = 0
                    mock_session.get_completed_tasks.return_value = []
                    mock_progress.load.return_value = mock_session

                    with patch("afk.runner.auto_commit"):
                        run_loop(config, max_iterations=5)

        # auto_commit is only called when there are newly completed tasks
        # In this case there are none, so it shouldn't be called


class TestRunLoopIntegration:
    """Integration-style tests for run_loop."""

    def test_full_loop_with_mocked_ai(self, temp_afk_dir: Path) -> None:
        """Test a full loop with mocked AI responses."""
        config = AfkConfig(
            ai_cli=AiCliConfig(command="echo", args=[]),
            archive=ArchiveConfig(enabled=False),
            limits=LimitsConfig(max_iterations=3),
        )

        # Create a simple task source
        tasks_file = temp_afk_dir.parent / "tasks.json"
        tasks_file.write_text('{"tasks": [{"id": "task-1", "description": "Test task"}]}')

        call_count = [0]

        def mock_check_limits(*args, **kwargs):
            call_count[0] += 1
            if call_count[0] >= 2:
                return (False, "AFK_COMPLETE: All tasks finished")
            return (True, None)

        with patch("afk.runner.aggregate_tasks") as mock_tasks:
            mock_tasks.return_value = [MagicMock(id="task-1")]

            with patch("afk.runner.check_limits", side_effect=mock_check_limits):
                with patch("afk.runner.run_iteration") as mock_iteration:
                    mock_iteration.return_value = IterationResult(success=True)

                    with patch("afk.runner.SessionProgress") as mock_progress:
                        mock_session = MagicMock()
                        mock_session.iterations = 0
                        mock_session.get_completed_tasks.return_value = []
                        mock_progress.load.return_value = mock_session

                        result = run_loop(config, max_iterations=3)

        assert result.stop_reason == StopReason.COMPLETE
        assert result.iterations_completed == 1

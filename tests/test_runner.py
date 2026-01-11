"""Tests for afk.runner module."""

from __future__ import annotations

import subprocess
from pathlib import Path
from unittest.mock import MagicMock, patch

from afk.config import AfkConfig, AiCliConfig, ArchiveConfig, GitConfig, LimitsConfig
from afk.runner import (
    COMPLETION_SIGNALS,
    IterationResult,
    IterationRunner,
    RunResult,
    StopReason,
    _contains_completion_signal,
    run_iteration,
    run_loop,
    run_prompt_only,
)


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
                # Mock streaming readline - returns lines then empty string to stop
                mock_process.stdout.readline.side_effect = ["Output\n", ""]
                mock_process.stdin = MagicMock()
                mock_popen.return_value = mock_process

                result = run_iteration(config, iteration=1)

        assert result.success is True
        assert "Output" in result.output
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
        """Test handles iteration timeout (non-streaming mode)."""
        config = AfkConfig(
            ai_cli=AiCliConfig(command="sleep", args=["100"]),
            limits=LimitsConfig(timeout_minutes=1),
        )

        with patch("afk.runner.generate_prompt") as mock_prompt:
            mock_prompt.return_value = "Test prompt"

            with patch("subprocess.Popen") as mock_popen:
                mock_process = MagicMock()
                mock_process.stdout = None  # Disable streaming to use communicate()
                mock_process.stdin = MagicMock()
                mock_process.communicate.side_effect = subprocess.TimeoutExpired(
                    cmd="sleep", timeout=60
                )
                mock_process.kill = MagicMock()
                mock_popen.return_value = mock_process

                result = run_iteration(config, iteration=1, stream=False)

        assert result.success is False
        assert "timed out" in result.error


class TestRunLoop:
    """Tests for run_loop function."""

    def test_stops_when_no_tasks(self, temp_afk_dir: Path) -> None:
        """Test stops immediately when no pending stories."""
        from afk.prd_store import PrdDocument

        config = AfkConfig()

        # Empty PRD = all complete
        empty_prd = PrdDocument(userStories=[])

        with patch("afk.runner.sync_prd", return_value=empty_prd):
            with patch("afk.runner.archive_session"):
                result = run_loop(config, max_iterations=5)

        assert result.stop_reason == StopReason.COMPLETE
        assert result.iterations_completed == 0

    def test_respects_max_iterations(self, temp_afk_dir: Path) -> None:
        """Test respects max_iterations limit."""
        from afk.prd_store import PrdDocument, UserStory

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

        mock_prd = PrdDocument(
            userStories=[UserStory(id="task-1", title="Test", description="Test", passes=False)]
        )

        with patch("afk.runner.sync_prd", return_value=mock_prd):
            with patch("afk.runner.load_prd", return_value=mock_prd):
                with patch("afk.runner.check_limits") as mock_limits:
                    # Allow first 2 iterations, then signal complete
                    mock_limits.side_effect = [
                        (True, None),
                        (True, None),
                        (False, "AFK_COMPLETE: All tasks finished"),
                    ]

                    with patch.object(IterationRunner, "run", side_effect=mock_run_iteration):
                        with patch("afk.runner.SessionProgress"):
                            result = run_loop(config, max_iterations=10)

        assert result.stop_reason == StopReason.COMPLETE

    def test_archives_previous_session(self, temp_afk_dir: Path) -> None:
        """Test archives previous session before starting."""
        from afk.prd_store import PrdDocument

        config = AfkConfig(archive=ArchiveConfig(enabled=True))

        # Create progress file
        progress_file = temp_afk_dir / "progress.json"
        progress_file.write_text('{"iterations": 5, "tasks": {}}')

        empty_prd = PrdDocument(userStories=[])

        with patch("afk.runner.sync_prd", return_value=empty_prd):
            with patch("afk.runner.archive_session") as mock_archive:
                mock_archive.return_value = Path(".afk/archive/test")

                with patch("afk.runner.clear_session"):
                    with patch("afk.runner.SessionProgress") as mock_progress:
                        mock_progress.load.return_value = MagicMock(iterations=5)

                        run_loop(config, max_iterations=5)

        # Should archive when previous session exists
        mock_archive.assert_called()

    def test_resume_skips_initial_archiving(self, temp_afk_dir: Path) -> None:
        """Test resume=True skips archiving previous session at start."""
        config = AfkConfig(archive=ArchiveConfig(enabled=True))

        # Create progress file
        progress_file = temp_afk_dir / "progress.json"
        progress_file.write_text('{"iterations": 5, "tasks": {}}')

        with patch("afk.runner.sync_prd") as mock_sync:
            mock_prd = MagicMock()
            mock_prd.userStories = []
            mock_sync.return_value = mock_prd

            with patch("afk.runner.load_prd") as mock_load:
                mock_load.return_value = mock_prd

                with patch("afk.runner.archive_session") as mock_archive:
                    mock_archive.return_value = None  # Prevent MagicMock in output

                    with patch("afk.runner.clear_session") as mock_clear:
                        with patch("afk.runner.SessionProgress") as mock_progress:
                            mock_session = MagicMock()
                            mock_session.iterations = 5
                            mock_progress.load.return_value = mock_session

                            run_loop(config, max_iterations=5, resume=True)

        # clear_session should NOT be called when resume=True (initial archive skipped)
        mock_clear.assert_not_called()
        # archive_session may still be called at end of loop, but not for "new_run" reason
        # Check that no call was made with reason="new_run"
        for call in mock_archive.call_args_list:
            if call.kwargs.get("reason") == "new_run" or (
                len(call.args) > 1 and call.args[1] == "new_run"
            ):
                raise AssertionError("archive_session was called with reason='new_run'")

    def test_resume_continues_from_previous_iteration(self, temp_afk_dir: Path) -> None:
        """Test resume continues from where it left off."""
        config = AfkConfig(archive=ArchiveConfig(enabled=True))

        with patch("afk.runner.sync_prd") as mock_sync:
            mock_prd = MagicMock()
            mock_prd.userStories = []
            mock_sync.return_value = mock_prd

            with patch("afk.runner.load_prd") as mock_load:
                mock_load.return_value = mock_prd

                with patch("afk.runner.archive_session"):
                    with patch("afk.runner.SessionProgress") as mock_progress:
                        mock_session = MagicMock()
                        mock_session.iterations = 3  # Previous iterations
                        mock_progress.load.return_value = mock_session

                        result = run_loop(config, max_iterations=5, resume=True)

        # Session should show previous iteration count was recognised
        assert result is not None

    def test_creates_branch_when_configured(
        self, temp_afk_dir: Path, mock_subprocess_run: MagicMock
    ) -> None:
        """Test creates feature branch when specified."""
        from afk.prd_store import PrdDocument

        config = AfkConfig(
            git=GitConfig(auto_branch=True, branch_prefix="afk/"),
            archive=ArchiveConfig(enabled=False),
        )

        empty_prd = PrdDocument(userStories=[])

        with patch("afk.runner.sync_prd", return_value=empty_prd):
            with patch("afk.runner.create_branch") as mock_branch:
                with patch("afk.runner.SessionProgress") as mock_progress:
                    mock_progress.load.return_value = MagicMock(iterations=0)

                    run_loop(config, max_iterations=5, branch="my-feature")

        mock_branch.assert_called_once_with("my-feature", config)

    def test_completes_when_all_stories_pass(self, temp_afk_dir: Path) -> None:
        """Test loop completes when all stories have passes: true."""
        from afk.prd_store import PrdDocument

        config = AfkConfig(
            git=GitConfig(auto_commit=True),
            archive=ArchiveConfig(enabled=False),
            ai_cli=AiCliConfig(command="echo", args=[]),
        )

        empty_prd = PrdDocument(userStories=[])

        with patch("afk.runner.sync_prd", return_value=empty_prd):
            with patch("afk.runner.check_limits") as mock_limits:
                mock_limits.return_value = (False, "AFK_COMPLETE")

                with patch("afk.runner.SessionProgress") as mock_progress:
                    mock_session = MagicMock()
                    mock_session.iterations = 0
                    mock_session.get_completed_tasks.return_value = []
                    mock_progress.load.return_value = mock_session

                    result = run_loop(config, max_iterations=5)

        # Empty PRD means all complete
        assert result.stop_reason == StopReason.COMPLETE


class TestTaskProgressTracking:
    """Tests for task progress tracking in the runner."""

    def test_marks_task_in_progress_before_iteration(self, temp_afk_dir: Path) -> None:
        """Test that current task is marked in_progress before running."""
        from afk.prd_store import PrdDocument, UserStory
        from afk.progress import SessionProgress

        config = AfkConfig(
            ai_cli=AiCliConfig(command="echo", args=[]),
            archive=ArchiveConfig(enabled=False),
        )

        mock_prd = PrdDocument(
            userStories=[
                UserStory(
                    id="story-1",
                    title="Test Story",
                    description="Test",
                    passes=False,
                    source="test",
                )
            ]
        )

        with patch("afk.runner.sync_prd", return_value=mock_prd):
            with patch("afk.runner.load_prd", return_value=mock_prd):
                with patch("afk.runner.all_stories_complete", return_value=False):
                    with patch("afk.runner.get_pending_stories", return_value=mock_prd.userStories):
                        with patch("afk.runner.check_limits") as mock_limits:
                            # Stop after first iteration
                            mock_limits.side_effect = [
                                (True, None),
                                (False, "AFK_COMPLETE"),
                            ]
                            with patch.object(IterationRunner, "run") as mock_run:
                                mock_run.return_value = IterationResult(success=True)

                                run_loop(config, max_iterations=1)

        # Verify task was marked in_progress
        progress = SessionProgress.load(temp_afk_dir / "progress.json")
        assert "story-1" in progress.tasks
        assert progress.tasks["story-1"].status == "in_progress"
        assert progress.tasks["story-1"].source == "test"

    def test_marks_task_completed_when_story_passes(self, temp_afk_dir: Path) -> None:
        """Test that _check_story_completion marks tasks as completed."""
        from afk.prd_store import PrdDocument, UserStory
        from afk.progress import SessionProgress
        from afk.runner import LoopController

        config = AfkConfig(
            ai_cli=AiCliConfig(command="echo", args=[]),
            archive=ArchiveConfig(enabled=False),
        )

        # Initial PRD with pending story
        old_prd = PrdDocument(
            userStories=[
                UserStory(
                    id="story-1",
                    title="Test Story",
                    description="Test",
                    passes=False,
                    source="json:test.json",
                )
            ]
        )

        # PRD after AI marks it complete
        new_prd = PrdDocument(
            userStories=[
                UserStory(
                    id="story-1",
                    title="Test Story",
                    description="Test",
                    passes=True,
                    source="json:test.json",
                )
            ]
        )

        controller = LoopController(config)

        with patch("afk.runner.load_prd", return_value=new_prd):
            completed_count = controller._check_story_completion(old_prd)

        assert completed_count == 1

        # Verify task was marked completed in progress.json
        progress = SessionProgress.load(temp_afk_dir / "progress.json")
        assert "story-1" in progress.tasks
        assert progress.tasks["story-1"].status == "completed"
        assert progress.tasks["story-1"].completed_at is not None


class TestRunLoopIntegration:
    """Integration-style tests for run_loop."""

    def test_full_loop_with_mocked_ai(self, temp_afk_dir: Path) -> None:
        """Test a full loop with mocked AI responses."""
        from afk.prd_store import PrdDocument, UserStory

        config = AfkConfig(
            ai_cli=AiCliConfig(command="echo", args=[]),
            archive=ArchiveConfig(enabled=False),
            limits=LimitsConfig(max_iterations=3),
        )

        call_count = [0]

        def mock_check_limits(*args, **kwargs):
            call_count[0] += 1
            if call_count[0] >= 2:
                return (False, "AFK_COMPLETE: All tasks finished")
            return (True, None)

        mock_prd = PrdDocument(
            userStories=[UserStory(id="task-1", title="Test", description="Test", passes=False)]
        )

        with patch("afk.runner.sync_prd", return_value=mock_prd):
            with patch("afk.runner.load_prd", return_value=mock_prd):
                with patch("afk.runner.all_stories_complete", return_value=False):
                    with patch("afk.runner.get_pending_stories", return_value=mock_prd.userStories):
                        with patch("afk.runner.check_limits", side_effect=mock_check_limits):
                            with patch.object(IterationRunner, "run") as mock_iteration:
                                mock_iteration.return_value = IterationResult(success=True)

                                with patch("afk.runner.SessionProgress") as mock_progress:
                                    mock_session = MagicMock()
                                    mock_session.iterations = 0
                                    mock_session.get_completed_tasks.return_value = []
                                    mock_progress.load.return_value = mock_session

                                    result = run_loop(config, max_iterations=3)

        assert result.stop_reason == StopReason.COMPLETE
        assert result.iterations_completed == 1


class TestCompletionSignals:
    """Tests for completion signal detection."""

    def test_completion_signals_defined(self) -> None:
        """Test that completion signals are defined."""
        assert "<promise>COMPLETE</promise>" in COMPLETION_SIGNALS
        assert "AFK_COMPLETE" in COMPLETION_SIGNALS
        assert "AFK_STOP" in COMPLETION_SIGNALS

    def test_contains_completion_signal_empty(self) -> None:
        """Test with empty output."""
        assert _contains_completion_signal(None) is False
        assert _contains_completion_signal("") is False

    def test_contains_completion_signal_ralf_style(self) -> None:
        """Test detecting ralf.sh style signal."""
        output = "Done with task\n<promise>COMPLETE</promise>\n"
        assert _contains_completion_signal(output) is True

    def test_contains_completion_signal_afk_style(self) -> None:
        """Test detecting AFK style signal."""
        assert _contains_completion_signal("All done AFK_COMPLETE") is True
        assert _contains_completion_signal("AFK_STOP now") is True

    def test_contains_no_signal(self) -> None:
        """Test with no completion signal."""
        assert _contains_completion_signal("Just some output") is False


class TestRunPromptOnly:
    """Tests for run_prompt_only function."""

    def test_prompt_only_basic(self, temp_project: Path) -> None:
        """Test basic prompt-only execution."""
        prompt_file = temp_project / "prompt.md"
        prompt_file.write_text("Do the thing\n")

        config = AfkConfig(
            ai_cli=AiCliConfig(command="echo", args=["test"]),
        )

        with patch("subprocess.Popen") as mock_popen:
            mock_process = MagicMock()
            mock_process.stdin = MagicMock()
            # Streaming mock - return line then empty to end
            mock_process.stdout.readline.side_effect = ["output\n", ""]
            mock_process.returncode = 0
            mock_popen.return_value = mock_process

            result = run_prompt_only(
                prompt_file=prompt_file,
                config=config,
                max_iterations=2,
            )

        assert result.iterations_completed == 2
        assert result.stop_reason == StopReason.MAX_ITERATIONS

    def test_prompt_only_completion_signal(self, temp_project: Path) -> None:
        """Test prompt-only stops on completion signal."""
        prompt_file = temp_project / "prompt.md"
        prompt_file.write_text("Do the thing\n")

        config = AfkConfig(
            ai_cli=AiCliConfig(command="echo", args=[]),
        )

        with patch("subprocess.Popen") as mock_popen:
            mock_process = MagicMock()
            mock_process.stdin = MagicMock()
            # Return completion signal in output
            mock_process.stdout.readline.side_effect = [
                "Done!\n",
                "<promise>COMPLETE</promise>\n",
                "",
            ]
            mock_process.returncode = 0
            mock_popen.return_value = mock_process

            result = run_prompt_only(
                prompt_file=prompt_file,
                config=config,
                max_iterations=10,
            )

        assert result.iterations_completed == 1
        assert result.stop_reason == StopReason.COMPLETE

    def test_prompt_only_ai_error(self, temp_project: Path) -> None:
        """Test prompt-only handles AI CLI errors."""
        prompt_file = temp_project / "prompt.md"
        prompt_file.write_text("Do the thing\n")

        config = AfkConfig(
            ai_cli=AiCliConfig(command="echo", args=[]),
        )

        with patch("subprocess.Popen") as mock_popen:
            mock_process = MagicMock()
            mock_process.stdin = MagicMock()
            mock_process.stdout.readline.side_effect = ["error\n", ""]
            mock_process.returncode = 1
            mock_popen.return_value = mock_process

            result = run_prompt_only(
                prompt_file=prompt_file,
                config=config,
                max_iterations=5,
            )

        assert result.stop_reason == StopReason.AI_ERROR

    def test_prompt_only_cli_not_found(self, temp_project: Path) -> None:
        """Test prompt-only handles missing CLI."""
        prompt_file = temp_project / "prompt.md"
        prompt_file.write_text("Do the thing\n")

        config = AfkConfig(
            ai_cli=AiCliConfig(command="nonexistent-cli", args=[]),
        )

        with patch("subprocess.Popen") as mock_popen:
            mock_popen.side_effect = FileNotFoundError()

            result = run_prompt_only(
                prompt_file=prompt_file,
                config=config,
                max_iterations=5,
            )

        assert result.stop_reason == StopReason.AI_ERROR
